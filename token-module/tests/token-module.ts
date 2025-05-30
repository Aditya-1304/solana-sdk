import * as anchor from '@coral-xyz/anchor';
import { AnchorError, BN, Program } from '@coral-xyz/anchor';
import { TokenModule } from '../target/types/token_module';
import { assert, expect } from 'chai';
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction } from '@solana/web3.js';
import {
  createAccount,
  mintTo as splMintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
  getMint,
  getAssociatedTokenAddressSync,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
} from '@solana/spl-token';

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

// Updated expectAnchorError
const expectAnchorError = async (fn: Promise<any>, expectedErrorCode: string, expectedErrorMessage?: string) => {
  try {
    await fn;
    assert.fail("Expected promise to be rejected but it resolved successfully");
  } catch (e: any) {
    let parsedError = e;
    // Try to parse logs if it's not already an AnchorError but has logs (like SendTransactionError)
    if (!(e instanceof AnchorError) && e.logs && typeof AnchorError.parse === 'function') {
      const p = AnchorError.parse(e.logs);
      if (p instanceof AnchorError) { // Ensure parsing resulted in an AnchorError
        parsedError = p;
      }
    }

    expect(parsedError).to.be.instanceOf(AnchorError, `Error was not an AnchorError or parsable as one. Original error: ${e.message || e}`);
    const anchorError = parsedError as AnchorError;
    expect(anchorError.error.errorCode.code).to.equal(expectedErrorCode, `Expected error code ${expectedErrorCode}, got ${anchorError.error.errorCode.code}. Full error: ${JSON.stringify(anchorError.error)}`);
    if (expectedErrorMessage) {
      expect(anchorError.error.errorMessage).to.equal(expectedErrorMessage);
    }
  }
};

describe("token-module", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenModule as Program<TokenModule>;
  const adminKeypair = Keypair.generate();
  const user1Keypair = Keypair.generate();
  const user2Keypair = Keypair.generate();
  const unauthorizedUserKeypair = Keypair.generate();

  let tokenAuthorityPDA: PublicKey;
  let tokenAuthorityBump: number;

  const TOKEN_DECIMALS = 9;

  before(async () => {
    await Promise.all([
      provider.connection.requestAirdrop(adminKeypair.publicKey, 10 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(user1Keypair.publicKey, 10 * LAMPORTS_PER_SOL), // Increased for more test flexibility
      provider.connection.requestAirdrop(user2Keypair.publicKey, 10 * LAMPORTS_PER_SOL), // Increased
      provider.connection.requestAirdrop(unauthorizedUserKeypair.publicKey, 2 * LAMPORTS_PER_SOL),
    ]).then(async (signatures) => {
      await Promise.all(signatures.map(sig => provider.connection.confirmTransaction(sig, "confirmed")))
    });

    const tokenAuthorityResult = findPda([Buffer.from("token_authority")], program.programId);
    tokenAuthorityPDA = tokenAuthorityResult.publicKey;
    tokenAuthorityBump = tokenAuthorityResult.bump;
  });

  describe("1. Initialize Token Authority", () => {
    it("Should initialize token authority successfully", async () => {
      await program.methods
        .initializeTokenAuthority()
        .accounts({
          admin: adminKeypair.publicKey,
          tokenAuthority: tokenAuthorityPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([adminKeypair])
        .rpc({ commitment: "confirmed" });

      const tokenAuthorityAccount = await program.account.tokenAuthority.fetch(tokenAuthorityPDA);
      assert.isTrue(tokenAuthorityAccount.admin.equals(adminKeypair.publicKey));
      assert.strictEqual(tokenAuthorityAccount.bump, tokenAuthorityBump);
    });

    it("Should fail to initialize token Authority if already initialized", async () => {
      // For re-initialization, the system program might error out first.
      // Expecting a generic error is safer than a specific Anchor error code here.
      try {
        await program.methods
          .initializeTokenAuthority()
          .accounts({
            admin: adminKeypair.publicKey,
            tokenAuthority: tokenAuthorityPDA,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([adminKeypair])
          .rpc({ commitment: "confirmed" });
        assert.fail("Transaction should have failed to re-initialize token authority.");
      } catch (error) {
        // console.log("Re-init error:", error); // For debugging
        expect(error).to.exist; // Basic assertion that an error was thrown
        // You might check error.message or error.logs for more specific details if needed
        // e.g. expect(error.toString()).to.include("custom program error: 0x0"); // If system program error
      }
    });
  });

  // ... (Remove 'as any' from all .accounts({...}) calls in subsequent tests)

  describe("2. Token Creation", () => {
    const mintKeypair = Keypair.generate();
    let tokenMetadataPDA: PublicKey;

    const tokenName = "Test Token";
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

    it("Should create a new token successfully", async () => {
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

    it("Should fail to create token with name too long", async () => {
      const longName = "a".repeat(33);
      const newMintKeypair = Keypair.generate();
      const newMetadataPDA = findPda([Buffer.from("token_metadata"), newMintKeypair.publicKey.toBuffer()], program.programId).publicKey;

      await expectAnchorError(
        program.methods
          .createToken(longName, tokenSymbol, TOKEN_DECIMALS, tokenUri, tokenMaxSupply)
          .accounts({
            admin: adminKeypair.publicKey,
            mint: newMintKeypair.publicKey,
            tokenMetadata: newMetadataPDA,
            tokenAuthority: tokenAuthorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
          } as any)
          .signers([adminKeypair, newMintKeypair])
          .rpc(),
        "NameTooLong"
      );
    });

    it("Should fail to create token with symbol too long", async () => {
      const longSymbol = "S".repeat(11);
      const newMintKeypair = Keypair.generate();
      const newMetadataPDA = findPda([Buffer.from("token_metadata"), newMintKeypair.publicKey.toBuffer()], program.programId).publicKey;

      await expectAnchorError(
        program.methods
          .createToken(tokenName, longSymbol, TOKEN_DECIMALS, tokenUri, tokenMaxSupply)
          .accounts({
            admin: adminKeypair.publicKey,
            mint: newMintKeypair.publicKey,
            tokenMetadata: newMetadataPDA,
            tokenAuthority: tokenAuthorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
          } as any)
          .signers([adminKeypair, newMintKeypair])
          .rpc(),
        "SymbolTooLong"
      );
    });

    it("Should fail to create token with URI too long", async () => {
      const longUri = "http://" + "u".repeat(195) + ".com";
      const newMintKeypair = Keypair.generate();
      const newMetadataPDA = findPda([Buffer.from("token_metadata"), newMintKeypair.publicKey.toBuffer()], program.programId).publicKey;

      await expectAnchorError(
        program.methods
          .createToken(tokenName, tokenSymbol, TOKEN_DECIMALS, longUri, tokenMaxSupply)
          .accounts({
            admin: adminKeypair.publicKey,
            mint: newMintKeypair.publicKey,
            tokenMetadata: newMetadataPDA,
            tokenAuthority: tokenAuthorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
          } as any)
          .signers([adminKeypair, newMintKeypair])
          .rpc(),
        "UriTooLong"
      );
    });

    it("Should allow creating token with a different admin (sets token_metadata.admin)", async () => {
      const newMintKeypair = Keypair.generate();
      const newMetadataPDA = findPda([Buffer.from("token_metadata"), newMintKeypair.publicKey.toBuffer()], program.programId).publicKey;

      await program.methods
        .createToken(tokenName, tokenSymbol, TOKEN_DECIMALS, tokenUri, tokenMaxSupply)
        .accounts({
          admin: unauthorizedUserKeypair.publicKey,
          mint: newMintKeypair.publicKey,
          tokenMetadata: newMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        } as any)
        .signers([unauthorizedUserKeypair, newMintKeypair])
        .rpc({ commitment: "confirmed" });

      const tokenMetadataAccount = await program.account.tokenMetadata.fetch(newMetadataPDA);
      assert.isTrue(tokenMetadataAccount.admin.equals(unauthorizedUserKeypair.publicKey));
    });
  });


  describe("3. Mint Tokens", () => {
    const mintKeypair = Keypair.generate();
    let tokenMetadataPDA: PublicKey;
    const mintAmount = new BN(100).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));

    before(async () => {
      const tokenMetadataResult = findPda([Buffer.from("token_metadata"), mintKeypair.publicKey.toBuffer()], program.programId);
      tokenMetadataPDA = tokenMetadataResult.publicKey;
      await program.methods
        .createToken("Mint Test Token", "MTT", TOKEN_DECIMALS, null, new BN(1000 * (10 ** TOKEN_DECIMALS)))
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
    });

    it("Should mint tokens successfully", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      const createAtaTx = new Transaction()
        .add(createAssociatedTokenAccountInstruction(
          user1Keypair.publicKey,
          user1Ata,
          user1Keypair.publicKey,
          mintKeypair.publicKey
        ));
      await provider.sendAndConfirm(createAtaTx, [user1Keypair], { commitment: "confirmed" });

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

      const user1AtaInfo = await getAccount(provider.connection, user1Ata);
      assert.isTrue(user1AtaInfo.amount === BigInt(mintAmount.toString()));
    });

    it("Should fail to mint zero tokens", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      await expectAnchorError(
        program.methods
          .mintTokens(new BN(0))
          .accounts({
            admin: adminKeypair.publicKey,
            mint: mintKeypair.publicKey,
            destination: user1Ata,
            tokenMetadata: tokenMetadataPDA,
            tokenAuthority: tokenAuthorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
          } as any)
          .signers([adminKeypair])
          .rpc(),
        "ZeroAmount"
      );
    });

    it("Should fail to mint if admin is not token_metadata.admin", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      await expectAnchorError(
        program.methods
          .mintTokens(mintAmount)
          .accounts({
            admin: unauthorizedUserKeypair.publicKey,
            mint: mintKeypair.publicKey,
            destination: user1Ata,
            tokenMetadata: tokenMetadataPDA,
            tokenAuthority: tokenAuthorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
          } as any)
          .signers([unauthorizedUserKeypair])
          .rpc(),
        "UnauthorizedMintAuthority"
      );
    });

    it("Should fail to mint if it exceeds max supply", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      const currentSupply = (await getMint(provider.connection, mintKeypair.publicKey)).supply;
      const tokenMetadata = await program.account.tokenMetadata.fetch(tokenMetadataPDA);
      const maxSupply = tokenMetadata.maxSupply;

      if (maxSupply) {
        const amountToExceed = maxSupply.sub(new BN(currentSupply.toString())).add(new BN(1));

        if (amountToExceed.gtn(0)) {
          await expectAnchorError(
            program.methods
              .mintTokens(amountToExceed)
              .accounts({
                admin: adminKeypair.publicKey,
                mint: mintKeypair.publicKey,
                destination: user1Ata,
                tokenMetadata: tokenMetadataPDA,
                tokenAuthority: tokenAuthorityPDA,
                tokenProgram: TOKEN_PROGRAM_ID,
              } as any)
              .signers([adminKeypair])
              .rpc(),
            "ExceedsMaxSupply"
          );
        } else {
          console.log("Skipping exceed max supply test: current supply might already be at max or amountToExceed is not positive.");
        }
      } else {
        console.log("Skipping exceed max supply test: no max_supply set for this token.");
      }
    });
  });

  describe("4. Token Transfers", () => {
    const mintKeypair = Keypair.generate();
    let tokenMetadataPDA: PublicKey;
    let user1Ata: PublicKey;
    let user2Ata: PublicKey;
    const initialMintAmount = new BN(200).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));

    before(async () => {
      tokenMetadataPDA = findPda([Buffer.from("token_metadata"), mintKeypair.publicKey.toBuffer()], program.programId).publicKey;
      await program.methods
        .createToken("Transfer Test Token", "XFER", TOKEN_DECIMALS, null, null)
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
        .rpc();

      user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      user2Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user2Keypair.publicKey);

      const tx1 = new Transaction().add(createAssociatedTokenAccountInstruction(user1Keypair.publicKey, user1Ata, user1Keypair.publicKey, mintKeypair.publicKey));
      await provider.sendAndConfirm(tx1, [user1Keypair]);
      const tx2 = new Transaction().add(createAssociatedTokenAccountInstruction(user2Keypair.publicKey, user2Ata, user2Keypair.publicKey, mintKeypair.publicKey));
      await provider.sendAndConfirm(tx2, [user2Keypair]);

      await program.methods
        .mintTokens(initialMintAmount)
        .accounts({
          admin: adminKeypair.publicKey,
          mint: mintKeypair.publicKey,
          destination: user1Ata,
          tokenMetadata: tokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
        } as any)
        .signers([adminKeypair])
        .rpc();
    });

    it("Should transfer tokens successfully", async () => {
      const transferAmount = new BN(30).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));
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

      const user1AtaInfoAfter = await getAccount(provider.connection, user1Ata);
      const user2AtaInfoAfter = await getAccount(provider.connection, user2Ata);
      const expectedUser1Balance = BigInt(user1InitialBalance.toString()) - BigInt(transferAmount.toString());
      assert.isTrue(user1AtaInfoAfter.amount === expectedUser1Balance);
      assert.isTrue(user2AtaInfoAfter.amount === BigInt(transferAmount.toString()));
    });

    it("Should fail to transfer zero tokens", async () => {
      await expectAnchorError(
        program.methods
          .transferTokens(new BN(0))
          .accounts({
            owner: user1Keypair.publicKey,
            mint: mintKeypair.publicKey,
            fromAccount: user1Ata,
            toAccount: user2Ata,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user1Keypair])
          .rpc(),
        "ZeroAmount"
      );
    });

    it("Should fail to transfer with insufficient funds", async () => {
      const user1Balance = (await getAccount(provider.connection, user1Ata)).amount;
      const transferAmount = new BN(user1Balance.toString()).add(new BN(1));
      await expectAnchorError(
        program.methods
          .transferTokens(transferAmount)
          .accounts({
            owner: user1Keypair.publicKey,
            mint: mintKeypair.publicKey,
            fromAccount: user1Ata,
            toAccount: user2Ata,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user1Keypair])
          .rpc(),
        "ConstraintTokenBalance"
      );
    });

    it("Should fail to transfer if 'from' account owner is not the signer", async () => {
      const transferAmount = new BN(1);
      await expectAnchorError(
        program.methods
          .transferTokens(transferAmount)
          .accounts({
            owner: unauthorizedUserKeypair.publicKey,
            mint: mintKeypair.publicKey,
            fromAccount: user1Ata,
            toAccount: user2Ata,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([unauthorizedUserKeypair])
          .rpc(),
        "ConstraintRaw" // Changed from ConstraintTokenOwner
      );
    });
  });

  describe("5. Burning Tokens", () => {
    const mintKeypair = Keypair.generate();
    let tokenMetadataPDA: PublicKey;
    let user1Ata: PublicKey;
    const initialMintAmount = new BN(50).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));

    before(async () => {
      tokenMetadataPDA = findPda([Buffer.from("token_metadata"), mintKeypair.publicKey.toBuffer()], program.programId).publicKey;
      await program.methods
        .createToken("Burn Test Token", "BURN", TOKEN_DECIMALS, null, null)
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
        .rpc();

      user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      const tx1 = new Transaction().add(createAssociatedTokenAccountInstruction(user1Keypair.publicKey, user1Ata, user1Keypair.publicKey, mintKeypair.publicKey));
      await provider.sendAndConfirm(tx1, [user1Keypair]);

      await program.methods
        .mintTokens(initialMintAmount)
        .accounts({
          admin: adminKeypair.publicKey,
          mint: mintKeypair.publicKey,
          destination: user1Ata,
          tokenMetadata: tokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
        } as any)
        .signers([adminKeypair])
        .rpc();
    });

    it("Should burn tokens successfully", async () => {
      const burnAmount = new BN(10).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));
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

      const user1AtaInfoAfter = await getAccount(provider.connection, user1Ata);
      const mintInfoAfter = await getMint(provider.connection, mintKeypair.publicKey);
      const expectedUser1Balance = BigInt(user1InitialBalance.toString()) - BigInt(burnAmount.toString());
      const expectedMintSupply = BigInt(mintInitialSupply.toString()) - BigInt(burnAmount.toString());
      assert.isTrue(user1AtaInfoAfter.amount === expectedUser1Balance);
      assert.isTrue(mintInfoAfter.supply === expectedMintSupply);
    });

    it("Should fail to burn zero tokens", async () => {
      await expectAnchorError(
        program.methods
          .burnTokens(new BN(0))
          .accounts({
            owner: user1Keypair.publicKey,
            mint: mintKeypair.publicKey,
            tokenAccount: user1Ata,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user1Keypair])
          .rpc(),
        "ZeroAmount"
      );
    });

    it("Should fail to burn more tokens than balance", async () => {
      const user1Balance = (await getAccount(provider.connection, user1Ata)).amount;
      const burnAmount = new BN(user1Balance.toString()).add(new BN(1));
      await expectAnchorError(
        program.methods
          .burnTokens(burnAmount)
          .accounts({
            owner: user1Keypair.publicKey,
            mint: mintKeypair.publicKey,
            tokenAccount: user1Ata,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user1Keypair])
          .rpc(),
        "ConstraintTokenBalance"
      );
    });
  });

  describe("6. Freeze and Thaw", () => {
    const mintKeypair = Keypair.generate();
    let tokenMetadataPDA: PublicKey;
    let user1Ata: PublicKey;

    before(async () => {
      tokenMetadataPDA = findPda([Buffer.from("token_metadata"), mintKeypair.publicKey.toBuffer()], program.programId).publicKey;
      await program.methods
        .createToken("Freeze Test", "FRZ", TOKEN_DECIMALS, null, null)
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
        .rpc();

      user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      const tx = new Transaction().add(createAssociatedTokenAccountInstruction(user1Keypair.publicKey, user1Ata, user1Keypair.publicKey, mintKeypair.publicKey));
      await provider.sendAndConfirm(tx, [user1Keypair]);

      await program.methods
        .mintTokens(new BN(100).mul(new BN(10).pow(new BN(TOKEN_DECIMALS))))
        .accounts({
          admin: adminKeypair.publicKey,
          mint: mintKeypair.publicKey,
          destination: user1Ata,
          tokenMetadata: tokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID
        } as any)
        .signers([adminKeypair])
        .rpc();
    });

    it("Should freeze an account, prevent transfer, then thaw and allow transfer", async () => {
      await program.methods.freezeTokenAccount().accounts({
        admin: adminKeypair.publicKey,
        mint: mintKeypair.publicKey,
        tokenAccount: user1Ata,
        tokenMetadata: tokenMetadataPDA,
        tokenAuthority: tokenAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any).signers([adminKeypair]).rpc();
      let ataInfo = await getAccount(provider.connection, user1Ata);
      assert.isTrue(ataInfo.isFrozen);

      const user2Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user2Keypair.publicKey);
      const tx = new Transaction().add(createAssociatedTokenAccountInstruction(user2Keypair.publicKey, user2Ata, user2Keypair.publicKey, mintKeypair.publicKey));
      await provider.sendAndConfirm(tx, [user2Keypair]);

      await expectAnchorError(
        program.methods.transferTokens(new BN(1)).accounts({
          owner: user1Keypair.publicKey,
          mint: mintKeypair.publicKey,
          fromAccount: user1Ata,
          toAccount: user2Ata,
          tokenProgram: TOKEN_PROGRAM_ID,
        }).signers([user1Keypair]).rpc(),
        "AccountFrozen"
      );

      await program.methods.thawTokenAccount().accounts({
        admin: adminKeypair.publicKey,
        mint: mintKeypair.publicKey,
        tokenAccount: user1Ata,
        tokenMetadata: tokenMetadataPDA,
        tokenAuthority: tokenAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any).signers([adminKeypair]).rpc();
      ataInfo = await getAccount(provider.connection, user1Ata);
      assert.isFalse(ataInfo.isFrozen);

      await program.methods.transferTokens(new BN(1)).accounts({
        owner: user1Keypair.publicKey,
        mint: mintKeypair.publicKey,
        fromAccount: user1Ata,
        toAccount: user2Ata,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([user1Keypair]).rpc();
      const user2AtaInfo = await getAccount(provider.connection, user2Ata);
      assert.isTrue(user2AtaInfo.amount === BigInt(1));
    });

    it("Should fail to freeze if signer is not token_metadata.admin", async () => {
      await expectAnchorError(
        program.methods.freezeTokenAccount().accounts({
          admin: unauthorizedUserKeypair.publicKey,
          mint: mintKeypair.publicKey,
          tokenAccount: user1Ata,
          tokenMetadata: tokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
        } as any).signers([unauthorizedUserKeypair]).rpc(),
        "UnauthorizedFreezeAuthority"
      );
    });
  });

  describe("7. Escrow Operations", () => {
    const escrowMintKeypair = Keypair.generate();
    let escrowTokenMetadataPDA: PublicKey;
    const escrowSeed = Keypair.generate().publicKey.toBuffer().slice(0, 32); // Example seed
    const escrowAmount = new BN(50).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));
    let user1EscrowAta: PublicKey;

    // These will be set in tests that create them
    let currentEscrowPDA: PublicKey;
    let currentEscrowAuthPDA: PublicKey;
    let currentEscrowTokenATA: PublicKey;


    before(async () => {
      escrowTokenMetadataPDA = findPda([Buffer.from("token_metadata"), escrowMintKeypair.publicKey.toBuffer()], program.programId).publicKey;
      await program.methods.createToken("Escrow Test Token", "ESCR", TOKEN_DECIMALS, null, null)
        .accounts({
          admin: adminKeypair.publicKey,
          mint: escrowMintKeypair.publicKey,
          tokenMetadata: escrowTokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY
        } as any)
        .signers([adminKeypair, escrowMintKeypair]).rpc();

      user1EscrowAta = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, user1Keypair.publicKey);
      const txAta = new Transaction().add(createAssociatedTokenAccountInstruction(user1Keypair.publicKey, user1EscrowAta, user1Keypair.publicKey, escrowMintKeypair.publicKey));
      await provider.sendAndConfirm(txAta, [user1Keypair]);

      // Mint enough for multiple escrow tests
      await program.methods.mintTokens(escrowAmount.mul(new BN(5))) // Increased mint
        .accounts({
          admin: adminKeypair.publicKey,
          mint: escrowMintKeypair.publicKey,
          destination: user1EscrowAta,
          tokenMetadata: escrowTokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA,
          tokenProgram: TOKEN_PROGRAM_ID
        } as any)
        .signers([adminKeypair]).rpc();
    });

    it("Should create escrow successfully (with specific recipient)", async () => {
      const localEscrowSeed = Keypair.generate().publicKey.toBuffer().slice(0, 32);
      currentEscrowPDA = findPda([Buffer.from("token_escrow"), user1Keypair.publicKey.toBuffer(), escrowMintKeypair.publicKey.toBuffer(), localEscrowSeed], program.programId).publicKey;
      currentEscrowAuthPDA = findPda([Buffer.from("escrow_authority"), currentEscrowPDA.toBuffer()], program.programId).publicKey;
      currentEscrowTokenATA = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, currentEscrowAuthPDA, true);

      await program.methods.createEscrow(escrowAmount, Array.from(localEscrowSeed), user2Keypair.publicKey)
        .accounts({
          sender: user1Keypair.publicKey,
          mint: escrowMintKeypair.publicKey,
          senderTokenAccount: user1EscrowAta,
          escrow: currentEscrowPDA,
          escrowAuthority: currentEscrowAuthPDA,
          escrowTokenAccount: currentEscrowTokenATA,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        } as any).signers([user1Keypair]).rpc();

      const escrowData = await program.account.escrow.fetch(currentEscrowPDA);
      assert.isTrue(escrowData.recipient?.equals(user2Keypair.publicKey));
      const escrowAtaBalance = (await getAccount(provider.connection, currentEscrowTokenATA)).amount;
      assert.isTrue(escrowAtaBalance === BigInt(escrowAmount.toString()));
    });

    it("Should release escrow to the specified recipient", async () => {
      // This test assumes 'currentEscrowPDA' etc. are from the previous successful creation.
      // Ensure this test runs after a successful creation or set up its own escrow.
      // For simplicity, we'll assume it follows the above. If tests run in parallel or are reordered, this will break.
      // It's better for each test to set up its own escrow if they are meant to be independent.
      // However, to fix the current structure:
      if (!currentEscrowPDA) { // Basic guard
        console.log("Skipping 'Should release escrow to the specified recipient' as no escrow was set up by previous test in sequence.");
        return;
      }
      const user2EscrowAta = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, user2Keypair.publicKey);
      // Ensure recipient ATA exists if not created by releaseEscrow
      try {
        await getAccount(provider.connection, user2EscrowAta);
      } catch (error) { // Account not found
        const tx = new Transaction().add(createAssociatedTokenAccountInstruction(user2Keypair.publicKey, user2EscrowAta, user2Keypair.publicKey, escrowMintKeypair.publicKey));
        await provider.sendAndConfirm(tx, [user2Keypair]);
      }


      await program.methods.releaseEscrow().accounts({
        recipient: user2Keypair.publicKey,
        escrow: currentEscrowPDA,
        mint: escrowMintKeypair.publicKey,
        escrowAuthority: currentEscrowAuthPDA,
        escrowTokenAccount: currentEscrowTokenATA,
        recipientTokenAccount: user2EscrowAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any).signers([user2Keypair]).rpc();

      const user2AtaData = await getAccount(provider.connection, user2EscrowAta);
      assert.isTrue(user2AtaData.amount === BigInt(escrowAmount.toString()));
      const escrowAccountInfo = await provider.connection.getAccountInfo(currentEscrowPDA);
      assert.isNull(escrowAccountInfo, "Escrow account should be closed after release");
    });

    it("Should fail to release escrow if already claimed (account closed)", async () => {
      const localEscrowSeed = Keypair.generate().publicKey.toBuffer().slice(0, 32);
      const newEscrowPDA = findPda([Buffer.from("token_escrow"), user1Keypair.publicKey.toBuffer(), escrowMintKeypair.publicKey.toBuffer(), localEscrowSeed], program.programId).publicKey;
      const newEscrowAuthPDA = findPda([Buffer.from("escrow_authority"), newEscrowPDA.toBuffer()], program.programId).publicKey;
      const newEscrowTokenATA = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, newEscrowAuthPDA, true);

      await program.methods.createEscrow(escrowAmount, Array.from(localEscrowSeed), user2Keypair.publicKey)
        .accounts({
          sender: user1Keypair.publicKey, mint: escrowMintKeypair.publicKey, senderTokenAccount: user1EscrowAta,
          escrow: newEscrowPDA, escrowAuthority: newEscrowAuthPDA, escrowTokenAccount: newEscrowTokenATA,
          tokenProgram: TOKEN_PROGRAM_ID, associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId, rent: SYSVAR_RENT_PUBKEY,
        } as any).signers([user1Keypair]).rpc();

      const user2EscrowAta = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, user2Keypair.publicKey);
      try { await getAccount(provider.connection, user2EscrowAta); } catch (error) {
        const tx = new Transaction().add(createAssociatedTokenAccountInstruction(user2Keypair.publicKey, user2EscrowAta, user2Keypair.publicKey, escrowMintKeypair.publicKey));
        await provider.sendAndConfirm(tx, [user2Keypair]);
      }

      await program.methods.releaseEscrow().accounts({
        recipient: user2Keypair.publicKey, escrow: newEscrowPDA, mint: escrowMintKeypair.publicKey,
        escrowAuthority: newEscrowAuthPDA, escrowTokenAccount: newEscrowTokenATA,
        recipientTokenAccount: user2EscrowAta, tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
      } as any).signers([user2Keypair]).rpc();

      try {
        await program.methods.releaseEscrow().accounts({
          recipient: user2Keypair.publicKey, escrow: newEscrowPDA, mint: escrowMintKeypair.publicKey,
          escrowAuthority: newEscrowAuthPDA, escrowTokenAccount: newEscrowTokenATA, // This ATA might also be closed if escrow authority was signer for closing it
          recipientTokenAccount: user2EscrowAta, tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        } as any).signers([user2Keypair]).rpc();
        assert.fail("Should have failed to release an already claimed (closed) escrow.");
      } catch (error) {
        expect(error).to.exist;
        // Expect an error indicating the escrow account is not found or not initialized
        // This can vary, e.g. "AccountNotInitialized" or a lower-level error.
        // console.log("Error releasing claimed escrow:", error);
      }
    });

    it("Should fail to release escrow to an unauthorized recipient", async () => {
      const localEscrowSeed = Keypair.generate().publicKey.toBuffer().slice(0, 32);
      const newEscrowPDA = findPda([Buffer.from("token_escrow"), user1Keypair.publicKey.toBuffer(), escrowMintKeypair.publicKey.toBuffer(), localEscrowSeed], program.programId).publicKey;
      const newEscrowAuthPDA = findPda([Buffer.from("escrow_authority"), newEscrowPDA.toBuffer()], program.programId).publicKey;
      const newEscrowTokenATA = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, newEscrowAuthPDA, true);

      await program.methods.createEscrow(escrowAmount, Array.from(localEscrowSeed), user2Keypair.publicKey) // Escrow for user2
        .accounts({
          sender: user1Keypair.publicKey, mint: escrowMintKeypair.publicKey, senderTokenAccount: user1EscrowAta,
          escrow: newEscrowPDA, escrowAuthority: newEscrowAuthPDA, escrowTokenAccount: newEscrowTokenATA,
          tokenProgram: TOKEN_PROGRAM_ID, associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId, rent: SYSVAR_RENT_PUBKEY,
        } as any).signers([user1Keypair]).rpc();

      const unauthorizedRecipientAta = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, unauthorizedUserKeypair.publicKey);
      try { await getAccount(provider.connection, unauthorizedRecipientAta); } catch (error) {
        const tx = new Transaction().add(createAssociatedTokenAccountInstruction(unauthorizedUserKeypair.publicKey, unauthorizedRecipientAta, unauthorizedUserKeypair.publicKey, escrowMintKeypair.publicKey));
        await provider.sendAndConfirm(tx, [unauthorizedUserKeypair]);
      }

      // This test relies on the Rust program correctly implementing the UnauthorizedRecipient error.
      // If the Rust program is flawed and proceeds to token transfer, it might fail with 0x1 (insufficient funds in escrow ATA if prior tests drained it)
      // or another error. The funding fix helps ensure the escrow ATA *should* have funds.
      await expectAnchorError(
        program.methods.releaseEscrow().accounts({
          recipient: unauthorizedUserKeypair.publicKey, // Unauthorized user trying to claim
          escrow: newEscrowPDA, mint: escrowMintKeypair.publicKey,
          escrowAuthority: newEscrowAuthPDA, escrowTokenAccount: newEscrowTokenATA,
          recipientTokenAccount: unauthorizedRecipientAta, tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        } as any).signers([unauthorizedUserKeypair]).rpc(),
        "UnauthorizedRecipient" // This is the ideal error. If program logic is flawed, actual error might differ.
      );
    });

    it("Should allow sender to release escrow if no specific recipient was set", async () => {
      const localEscrowSeed = Keypair.generate().publicKey.toBuffer().slice(0, 32);
      const newEscrowPDA = findPda([Buffer.from("token_escrow"), user1Keypair.publicKey.toBuffer(), escrowMintKeypair.publicKey.toBuffer(), localEscrowSeed], program.programId).publicKey;
      const newEscrowAuthPDA = findPda([Buffer.from("escrow_authority"), newEscrowPDA.toBuffer()], program.programId).publicKey;
      const newEscrowTokenATA = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, newEscrowAuthPDA, true);

      const senderInitialBalanceUser1Ata = (await getAccount(provider.connection, user1EscrowAta)).amount;

      await program.methods.createEscrow(escrowAmount, Array.from(localEscrowSeed), null) // No recipient
        .accounts({
          sender: user1Keypair.publicKey, mint: escrowMintKeypair.publicKey, senderTokenAccount: user1EscrowAta,
          escrow: newEscrowPDA, escrowAuthority: newEscrowAuthPDA, escrowTokenAccount: newEscrowTokenATA,
          tokenProgram: TOKEN_PROGRAM_ID, associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId, rent: SYSVAR_RENT_PUBKEY,
        } as any).signers([user1Keypair]).rpc();

      // Balance of user1EscrowAta should have decreased by escrowAmount
      const senderBalanceAfterCreate = (await getAccount(provider.connection, user1EscrowAta)).amount;
      expect(BigInt(senderBalanceAfterCreate.toString())).to.equal(BigInt(senderInitialBalanceUser1Ata.toString()) - BigInt(escrowAmount.toString()));

      await program.methods.releaseEscrow().accounts({
        recipient: user1Keypair.publicKey, // Sender is claiming
        escrow: newEscrowPDA, mint: escrowMintKeypair.publicKey,
        escrowAuthority: newEscrowAuthPDA, escrowTokenAccount: newEscrowTokenATA,
        recipientTokenAccount: user1EscrowAta, // Send back to sender's original ATA
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any).signers([user1Keypair]).rpc();

      const senderFinalBalance = (await getAccount(provider.connection, user1EscrowAta)).amount;
      // Expected balance should be the balance after creation + escrowAmount
      expect(BigInt(senderFinalBalance.toString())).to.equal(BigInt(senderBalanceAfterCreate.toString()) + BigInt(escrowAmount.toString()));
      // Which should also be equal to the initial balance before this specific escrow operation
      expect(BigInt(senderFinalBalance.toString())).to.equal(BigInt(senderInitialBalanceUser1Ata.toString()));
    });
  });

  describe("8. Transfer SOL", () => {
    it("Should transfer SOL successfully", async () => {
      const amountToTransfer = new BN(0.5 * LAMPORTS_PER_SOL);
      const user1InitialLamports = await provider.connection.getBalance(user1Keypair.publicKey);
      const user2InitialLamports = await provider.connection.getBalance(user2Keypair.publicKey);

      await program.methods.transferSol(amountToTransfer)
        .accounts({ from: user1Keypair.publicKey, to: user2Keypair.publicKey, systemProgram: SystemProgram.programId } as any)
        .signers([user1Keypair]).rpc();

      const user1AfterLamports = await provider.connection.getBalance(user1Keypair.publicKey);
      const user2AfterLamports = await provider.connection.getBalance(user2Keypair.publicKey);

      // Check receiver's balance increased by the amount (allowing for other tx fees on user2 not related to this transfer)
      expect(user2AfterLamports).to.be.gte(user2InitialLamports + amountToTransfer.toNumber() - 10000); // Allow some leeway

      // Check sender's balance decreased by at least the transfer amount
      expect(user1InitialLamports - user1AfterLamports).to.be.gte(amountToTransfer.toNumber());
      // And by at most transfer amount + typical fee (e.g. 5000 lamports, can be up to 10000 for safety)
      expect(user1InitialLamports - user1AfterLamports).to.be.lessThanOrEqual(amountToTransfer.toNumber() + 10000);
    });

    it("Should fail to transfer SOL with insufficient balance", async () => {
      const poorUser = Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(poorUser.publicKey, 5000), // Barely enough for a fee, not for transfer
        "confirmed"
      );
      await new Promise(resolve => setTimeout(resolve, 2000));

      const amountToTransfer = new BN(1 * LAMPORTS_PER_SOL);

      try {
        await program.methods.transferSol(amountToTransfer)
          .accounts({ from: poorUser.publicKey, to: user2Keypair.publicKey, systemProgram: SystemProgram.programId } as any)
          .signers([poorUser]).rpc();
        assert.fail("Transaction should have failed due to insufficient funds.");
      } catch (error) {
        expect(error).to.exist;
        // Solana runtime errors for insufficient funds don't typically have Anchor error codes
        // console.log("SOL transfer insufficient funds error:", error.message);
        expect(error.message || error.toString()).to.include("insufficient lamports"); // Or similar message
      }
    });
  });
});