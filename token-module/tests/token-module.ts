import * as anchor from '@coral-xyz/anchor';
import { BN, Program } from '@coral-xyz/anchor';
import { TokenModule } from '../target/types/token_module';
import { assert, expect } from 'chai';
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction } from '@solana/web3.js';
import { createMint, createAccount, mintTo, getAccount, TOKEN_PROGRAM_ID, getMint, getAssociatedTokenAddressSync, ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountInstruction } from '@solana/spl-token';

type PdaResult = {
  publicKey: PublicKey;
  bump: number;
};

const findPda = (seeds: (Buffer | Uint8Array)[], programId: PublicKey): PdaResult => {
  const [publicKey, bump] = PublicKey.findProgramAddressSync(seeds, programId);
  return {
    publicKey,
    bump
  }
};

describe("token-module", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenModule as Program<TokenModule>;
  const adminKeypair = Keypair.generate();
  const user1Keypair = Keypair.generate();
  const user2Keypair = Keypair.generate();


  let tokenAuthorityPDA: PublicKey;
  let tokenAuthorityBump: number;

  const TOKEN_DECIMALS = 9;

  before(async () => {
    await Promise.all([
      provider.connection.requestAirdrop(adminKeypair.publicKey, 10 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(user1Keypair.publicKey, 5 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(user2Keypair.publicKey, 5 * LAMPORTS_PER_SOL),
    ]).then(async (signatures) => {
      await Promise.all(signatures.map(sig => provider.connection.confirmTransaction(sig, "confirmed")))
    });

    const tokenAuthorityResult = findPda([Buffer.from("token_authority")], program.programId);
    tokenAuthorityPDA = tokenAuthorityResult.publicKey;
    tokenAuthorityBump = tokenAuthorityResult.bump;
  })

  it("1. Initialize Token Authority", async () => {
    await program.methods
      .initializeTokenAuthority()
      .accounts({
        admin: adminKeypair.publicKey,
        tokenAuthority: tokenAuthorityPDA,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([adminKeypair])
      .rpc({ commitment: "confirmed" });

    const tokenAuthorityAccount = await program.account.tokenAuthority.fetch(tokenAuthorityPDA)
    assert.isTrue(tokenAuthorityAccount.admin.equals(adminKeypair.publicKey));
    assert.strictEqual(tokenAuthorityAccount.bump, tokenAuthorityBump);
  });


  describe("Token Operations", () => {
    const mintKeypair = Keypair.generate();
    let tokenMetadataPDA: PublicKey;

    const tokenName = "Test Token"
    const tokenSymbol = "TTK";
    const tokenUri = "https://example.com/token-metadata.json";
    const tokenMaxSupply = new BN(1_000_000_000).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));

    before(async () => {
      const tokenMetadataResult = findPda(
        [Buffer.from("token_metadata"), mintKeypair.publicKey.toBuffer()],
        program.programId
      );
      tokenMetadataPDA = tokenMetadataResult.publicKey;
    });

    it("2. Create a new Token", async () => {
      await program.methods
        .createToken(tokenName, tokenSymbol, TOKEN_DECIMALS, tokenUri, tokenMaxSupply)
        .accounts({
          admin: adminKeypair.publicKey,
          mint: mintKeypair.publicKey,
          tokenMetadata: tokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        } as any)
        .signers([adminKeypair, mintKeypair])
        .rpc({ commitment: "confirmed" });

      const mintInfo = await getMint(provider.connection, mintKeypair.publicKey, "confirmed", TOKEN_PROGRAM_ID);
      assert.strictEqual(mintInfo.decimals, TOKEN_DECIMALS);
      assert.isTrue(mintInfo.mintAuthority?.equals(tokenAuthorityPDA));
      assert.isTrue(mintInfo.freezeAuthority?.equals(tokenAuthorityPDA));

      const tokenMetadataAccount = await program.account.tokenMetadata.fetch(tokenMetadataPDA);
      assert.strictEqual(tokenMetadataAccount.name, tokenName);
      assert.strictEqual(tokenMetadataAccount.symbol, tokenSymbol);
      assert.strictEqual(tokenMetadataAccount.decimals, TOKEN_DECIMALS);
      assert.strictEqual(tokenMetadataAccount.uri, tokenUri);
      assert.isTrue(tokenMetadataAccount.maxSupply.eq(tokenMaxSupply));
      assert.isTrue(tokenMetadataAccount.admin.equals(adminKeypair.publicKey));
      assert.isTrue(tokenMetadataAccount.mint.equals(mintKeypair.publicKey));
    });


    it("3. Mint Tokens", async () => {
      const user1Ata = getAssociatedTokenAddressSync(
        mintKeypair.publicKey,
        user1Keypair.publicKey,
        false,
        TOKEN_PROGRAM_ID
      )

      const createAtaTx = new Transaction().add(
        createAssociatedTokenAccountInstruction(
          user1Keypair.publicKey,
          user1Ata,
          user1Keypair.publicKey,
          mintKeypair.publicKey,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID,
        )
      );
      await provider.sendAndConfirm(createAtaTx, [user1Keypair], { commitment: "confirmed" });

      const mintAmount = new BN(100).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));

      const mintInfoBefore = await getMint(provider.connection, mintKeypair.publicKey);
      console.log("Mint Authority (from Mint Account):", mintInfoBefore.mintAuthority?.toString());
      console.log("Expected Token Authority PDA:", tokenAuthorityPDA.toString());

      await program.methods
        .mintTokens(mintAmount)
        .accounts({
          admin: adminKeypair.publicKey,
          mint: mintKeypair.publicKey,
          destination: user1Ata,
          tokenMetadata: tokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
        } as any)
        .signers([adminKeypair])
        .rpc({ commitment: "confirmed" });

      const user1AtaInfo = await getAccount(provider.connection, user1Ata, "confirmed", TOKEN_PROGRAM_ID);
      assert.isTrue(user1AtaInfo.amount === BigInt(mintAmount.toString()), `Expected ${mintAmount.toString()}, got ${user1AtaInfo.amount.toString()}`);

      const mintInfoAfter = await getMint(provider.connection, mintKeypair.publicKey, "confirmed", TOKEN_PROGRAM_ID);
      assert.isTrue(mintInfoAfter.supply === BigInt(mintAmount.toString()), `Expected ${mintAmount.toString()}, got ${mintInfoAfter.supply.toString()}`);
    });

    it("4. Transfer Tokens", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey, false, TOKEN_PROGRAM_ID);
      const user2Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user2Keypair.publicKey, false, TOKEN_PROGRAM_ID)

      const createAtaTx = new Transaction().add(
        createAssociatedTokenAccountInstruction(
          user2Keypair.publicKey,
          user2Ata,
          user2Keypair.publicKey,
          mintKeypair.publicKey,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID
        )
      );
      await provider.sendAndConfirm(createAtaTx, [user2Keypair], { commitment: "confirmed" })

      const transferAmount = new BN(30).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)))
      const user1InitialBalance = (await getAccount(provider.connection, user1Ata)).amount;

      await program.methods
        .transferTokens(transferAmount)
        .accounts({
          owner: user1Keypair.publicKey,
          mint: mintKeypair.publicKey,
          fromAccount: user1Ata,
          toAccount: user2Ata,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user1Keypair])
        .rpc({ commitment: "confirmed" });

      const user1AtaInfoAfter = await getAccount(provider.connection, user1Ata, "confirmed", TOKEN_PROGRAM_ID);

      const user2AtaInfoAfter = await getAccount(provider.connection, user2Ata, "confirmed", TOKEN_PROGRAM_ID);

      const expectedUser1Balance = BigInt(user1InitialBalance.toString()) - BigInt(transferAmount.toString());

      assert.isTrue(user1AtaInfoAfter.amount === expectedUser1Balance);
      assert.isTrue(user2AtaInfoAfter.amount === BigInt(transferAmount.toString()));
    });

    it("5. Burn Tokens", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey, false, TOKEN_PROGRAM_ID);

      const burnAmount = new BN(10).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)))

      const user1InitialBalance = (await getAccount(provider.connection, user1Ata)).amount;
      const mintInitialSupply = (await getMint(provider.connection, mintKeypair.publicKey)).supply;

      await program.methods
        .burnTokens(burnAmount)
        .accounts({
          owner: user1Keypair.publicKey,
          mint: mintKeypair.publicKey,
          tokenAccount: user1Ata,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user1Keypair])
        .rpc({ commitment: "confirmed" });

      const user1AtaInfoAfter = await getAccount(provider.connection, user1Ata, "confirmed", TOKEN_PROGRAM_ID);
      const mintInfoAfter = await getMint(provider.connection, mintKeypair.publicKey, "confirmed", TOKEN_PROGRAM_ID);

      const expectedUser1Balance = BigInt(user1InitialBalance.toString()) - BigInt(burnAmount.toString());
      const expectedMintSupply = BigInt(mintInitialSupply.toString()) - BigInt(burnAmount.toString());

      assert.isTrue(user1AtaInfoAfter.amount === expectedUser1Balance);
      assert.isTrue(mintInfoAfter.supply === expectedMintSupply)
    })

  })
})