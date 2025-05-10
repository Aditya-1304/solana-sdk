import * as anchor from '@coral-xyz/anchor';
import { AnchorError, BN, Program } from '@coral-xyz/anchor';
import { TokenModule } from '../target/types/token_module';
import { assert, expect } from 'chai';
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction } from '@solana/web3.js';
import { createMint as splCreateMint, createAccount, mintTo as splMintTo, getAccount, TOKEN_PROGRAM_ID, getMint, getAssociatedTokenAddressSync, ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountInstruction, Account as TokenAccountData, } from '@solana/spl-token';

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

const expectAnchorError = async (fn: Promise<any>, errorCode: string, errorMessage?: string) => {
  try {
    await fn;
    assert.fail("Expected promise to be rejected but it resolved successfully");
  } catch (err) {
    console.log("Caught error:", JSON.stringify(err, null, 2)); // For debugging error structure
    expect(err).to.be.instanceOf(AnchorError);
    const anchorError = err as AnchorError;
    expect(anchorError.error.errorCode.code).to.equal(errorCode, `Expected error code ${errorCode}`);
    if (errorMessage) {
      expect(anchorError.error.errorMessage).to.equal(errorMessage);
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
      provider.connection.requestAirdrop(user1Keypair.publicKey, 5 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(user2Keypair.publicKey, 5 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(unauthorizedUserKeypair.publicKey, 2 * LAMPORTS_PER_SOL),

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
  it("Should fail to initialize token Authority if already initialzed", async () => {
    await expectAnchorError(
      program.methods
        .initializeTokenAuthority()
        .accounts({
          admin: adminKeypair.publicKey,
          tokenAuthority: tokenAuthorityPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([adminKeypair])
        .rpc({ commitment: "confirmed" }),
      "AccountNotInitialized",
    )
  })


  describe("Token Creation", () => {
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
      const logName = "a".repeat(33);
      const newMintKeypair = Keypair.generate();
      const newMetadataPDA = findPda([Buffer.from("token_metadata"), newMintKeypair.publicKey.toBuffer()], program.programId).publicKey;

      await expectAnchorError(
        program.methods
          .createToken(logName, tokenSymbol, TOKEN_DECIMALS, tokenUri, tokenMaxSupply)
          .accounts({
            admin: adminKeypair.publicKey,
            mint: newMintKeypair.publicKey,
            tokenMetadata: newMetadataPDA,
            tokenAuthority: tokenAuthorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,

          } as any)
          .signers([adminKeypair, newMintKeypair])
          .rpc(),
        "TokenNameTooLong",
      )
    })

    it("Should fail to create token with symbol too long", async () => {
      const logSymbol = "S".repeat(11);
      const newMintKeypair = Keypair.generate();
      const newMetadataPDA = findPda([Buffer.from("token_metadata"), newMintKeypair.publicKey.toBuffer()], program.programId).publicKey;

      await expectAnchorError(
        program.methods
          .createToken(tokenName, logSymbol, TOKEN_DECIMALS, tokenUri, tokenMaxSupply)
          .accounts({
            admin: adminKeypair.publicKey,
            mint: newMintKeypair.publicKey,
            tokenMetadata: newMetadataPDA,
            tokenAuthority: tokenAuthorityPDA,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,

          } as any)
          .signers([adminKeypair, newMintKeypair])
          .rpc(),
        "TokenSymbolTooLong",
      )
    })
    it("Should fail to create token with URI too long", async () => {
      const longUri = "http://" + "u".repeat(195) + ".com"; // > 200
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

          } as any)
          .signers([adminKeypair, newMintKeypair])
          .rpc(),
        "TokenUriTooLong",
      )
    });

    it("Should fail to create token if admin signer is not the token authority admin", async () => {
      const newMintKeypair = Keypair.generate();
      const newMetadataPDA = findPda([Buffer.from("token_metadata"), newMintKeypair.publicKey.toBuffer()], program.programId).publicKey;

      await program.methods
        .createToken(tokenName, tokenSymbol, TOKEN_DECIMALS, tokenUri, tokenMaxSupply)
        .accounts({
          admin: unauthorizedUserKeypair.publicKey, // Different admin
          mint: newMintKeypair.publicKey,
          tokenMetadata: newMetadataPDA,
          tokenAuthority: tokenAuthorityPDA, // Still uses the global token authority for mint/freeze
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        } as any)
        .signers([unauthorizedUserKeypair, newMintKeypair]) // Unauthorized user signs
        .rpc({ commitment: "confirmed" });

      const tokenMetadataAccount = await program.account.tokenMetadata.fetch(newMetadataPDA);
      assert.isTrue(tokenMetadataAccount.admin.equals(unauthorizedUserKeypair.publicKey)); // New admin for this token's metadata
    });
  });



  describe("3. Mint Tokens", async () => {
    const mintKeypair = Keypair.generate();
    let tokenMetadataPDA: PublicKey;
    const mintAmount = new BN(100).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));

    before(async () => {
      const tokenMetadataResult = findPda([Buffer.from("token_metadata"), mintKeypair.publicKey.toBuffer()], program.programId);
      tokenMetadataPDA = tokenMetadataResult.publicKey;
      await program.methods
        .createToken("Mint Test Token", "MTT", TOKEN_DECIMALS, null, new BN(1000 * 10 ** TOKEN_DECIMALS)) // Max supply 1000
        .accounts({
          admin: adminKeypair.publicKey,
          mint: mintKeypair.publicKey,
          tokenMetadata: tokenMetadataPDA,
          tokenAuthority: tokenAuthorityPDA, tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId, rent: SYSVAR_RENT_PUBKEY,
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
        ))
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

    it("should fail to mint zero tokens", async () => {
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
        "MintAmountZero",
      )
    });

    it("Should fail to mint if admin is not token_metadata.admin", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      await expectAnchorError(
        program.methods
          .mintTokens(mintAmount)
          .accounts({
            admin: unauthorizedUserKeypair.publicKey, // Wrong admin
            mint: mintKeypair.publicKey, destination: user1Ata,
            tokenMetadata: tokenMetadataPDA, tokenAuthority: tokenAuthorityPDA, tokenProgram: TOKEN_PROGRAM_ID,
          } as any)
          .signers([unauthorizedUserKeypair])
          .rpc(),
        "UnauthorizedMintAuthority" // This comes from the `require` in `mint_tokens`
      );
    });

    it("Should fail to mint if it exceeds max supply", async () => {
      const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey);
      const currentSupply = (await getMint(provider.connection, mintKeypair.publicKey)).supply;
      const tokenMetadata = await program.account.tokenMetadata.fetch(tokenMetadataPDA);
      const maxSupply = tokenMetadata.maxSupply;
      const amountToExceed = maxSupply.sub(new BN(currentSupply.toString())).add(new BN(1)); // 1 more than remaining

      if (amountToExceed.gtn(0)) { // Only run if there's a max supply and we can exceed it
        await expectAnchorError(
          program.methods
            .mintTokens(amountToExceed)
            .accounts({
              admin: adminKeypair.publicKey, mint: mintKeypair.publicKey, destination: user1Ata,
              tokenMetadata: tokenMetadataPDA, tokenAuthority: tokenAuthorityPDA, tokenProgram: TOKEN_PROGRAM_ID,
            } as any)
            .signers([adminKeypair])
            .rpc(),
          "ExceedsMaxSupply"
        );
      } else {
        console.log("Skipping exceed max supply test as current supply might already be at max or no max_supply set.");
      }
    });

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
  });

  it("6. Freeze and Thaw Token Account", async () => {
    const user1Ata = getAssociatedTokenAddressSync(mintKeypair.publicKey, user1Keypair.publicKey, false, TOKEN_PROGRAM_ID);

    await program.methods
      .freezeTokenAccount()
      .accounts({
        admin: adminKeypair.publicKey,
        mint: mintKeypair.publicKey,
        tokenAccount: user1Ata,
        tokenMetadata: tokenMetadataPDA,
        tokenAuthority: tokenAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([adminKeypair])
      .rpc({ commitment: "confirmed" });

    let user1AtaInfo = await getAccount(provider.connection, user1Ata, "confirmed", TOKEN_PROGRAM_ID)
    assert.isTrue(user1AtaInfo.isFrozen);

    const user2ATa = getAssociatedTokenAddressSync(mintKeypair.publicKey, user2Keypair.publicKey, false, TOKEN_PROGRAM_ID);
    const transferAmount = new BN(1);

    try {
      await program.methods
        .transferTokens(transferAmount)
        .accounts({
          owner: user1Keypair.publicKey,
          mint: mintKeypair.publicKey,
          fromAccount: user1Ata,
          toAccount: user2ATa,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user1Keypair])
        .rpc({ commitment: "confirmed" })
      assert.fail("Transfer from frozen account should have failed");
    } catch (error) {
      expect(error.message).to.include("AccountFrozen");
    }

    await program.methods
      .thawTokenAccount()
      .accounts({
        admin: adminKeypair.publicKey,
        mint: mintKeypair.publicKey,
        tokenAccount: user1Ata,
        tokenMetadata: tokenMetadataPDA,
        tokenAuthority: tokenAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([adminKeypair])
      .rpc({ commitment: "confirmed" })
    user1AtaInfo = await getAccount(provider.connection, user1Ata, "confirmed", TOKEN_PROGRAM_ID);
    assert.isFalse(user1AtaInfo.isFrozen);

  })
});
describe("Escrow Operstions", () => {
  const escrowMintKeypair = Keypair.generate();
  let escrowTokenMetadataPDA: PublicKey;
  const escrowSeed = Keypair.generate().publicKey.toBuffer();

  let escrowPDA: PublicKey;
  let escrowAuthorityPDA: PublicKey;
  let escrowTokenAccount: PublicKey;

  const escrowAmount = new BN(50).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)));

  before(async () => {
    const escrowTokenMetadataResult = findPda(
      [Buffer.from("token_metadata"), escrowMintKeypair.publicKey.toBuffer()],
      program.programId
    );
    escrowTokenMetadataPDA = escrowTokenMetadataResult.publicKey;

    await program.methods
      .createToken("Escrow Token", "ESC", TOKEN_DECIMALS, null, null)
      .accounts({
        admin: adminKeypair.publicKey,
        mint: escrowMintKeypair.publicKey,
        tokenMetadata: escrowTokenMetadataPDA,
        tokenAuthority: tokenAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      } as any)
      .signers([adminKeypair, escrowMintKeypair])
      .rpc({ commitment: "confirmed" })

    const user1EscrowAta = getAssociatedTokenAddressSync(escrowMintKeypair.publicKey, user1Keypair.publicKey, false, TOKEN_PROGRAM_ID);

    const createAtaTx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        user1Keypair.publicKey, user1EscrowAta, user1Keypair.publicKey, escrowMintKeypair.publicKey, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(createAtaTx, [user1Keypair], { commitment: "confirmed" });

    const initialMintAmount = new BN(200).mul(new BN(10).pow(new BN(TOKEN_DECIMALS)))
    await program.methods
      .mintTokens(initialMintAmount)
      .accounts({
        admin: adminKeypair.publicKey,
        mint: escrowMintKeypair.publicKey,
        destination: user1EscrowAta,
        tokenMetadata: escrowTokenMetadataPDA,
        tokenAuthority: tokenAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      } as any)
      .signers([adminKeypair])
      .rpc({ commitment: "confirmed" })

    const escrowPdaResut = findPda(
      [
        Buffer.from("token_escrow"),
        user1Keypair.publicKey.toBuffer(),
        escrowMintKeypair.publicKey.toBuffer(),
        escrowSeed,
      ],
      program.programId
    );
    escrowPDA = escrowPdaResut.publicKey;

    const escrowAuthorityPdaResult = findPda(
      [Buffer.from("escrow_authority"), escrowPDA.toBuffer()],
      program.programId,
    );
    escrowAuthorityPDA = escrowAuthorityPdaResult.publicKey;

    escrowTokenAccount = getAssociatedTokenAddressSync(
      escrowMintKeypair.publicKey,
      escrowAuthorityPDA,
      true,
      TOKEN_PROGRAM_ID
    );
  });

  it("7. Create Escrow (with specific recipient)", async () => {
    const user1EscrowAta = getAssociatedTokenAddressSync(
      escrowMintKeypair.publicKey,
      user1Keypair.publicKey,
      false,
      TOKEN_PROGRAM_ID
    )
    await program.methods
      .createEscrow(escrowAmount, Array.from(escrowSeed), user2Keypair.publicKey)
      .accounts({
        sender: user1Keypair.publicKey,
        mint: escrowMintKeypair.publicKey,
        senderTokenAccount: user1EscrowAta,
        escrow: escrowPDA,
        escrowAuthority: escrowAuthorityPDA,
        escrowTokenAccount: escrowTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY
      } as any)
      .signers([user1Keypair])
      .rpc({ commitment: "confirmed" });

    const escrowAccountData = await program.account.escrow.fetch(escrowPDA);
    assert.isTrue(escrowAccountData.sender.equals(user1Keypair.publicKey));
    assert.isTrue(escrowAccountData.mint.equals(escrowMintKeypair.publicKey));
    assert.isTrue(escrowAccountData.amount.eq(escrowAmount));
    assert.isTrue(escrowAccountData.recipient?.equals(user2Keypair.publicKey));
    assert.isFalse(escrowAccountData.claimed);

    const escrowAtaInfo = await getAccount(provider.connection, escrowTokenAccount, 'confirmed', TOKEN_PROGRAM_ID);
    assert.isTrue(escrowAtaInfo.amount === BigInt(escrowAmount.toString()));

  });

  it("8. Release Escrow (to Specific recipient)", async () => {
    const user2EscrowAta = getAssociatedTokenAddressSync(
      escrowMintKeypair.publicKey, user2Keypair.publicKey, false, TOKEN_PROGRAM_ID
    );

    const user2InitialBalance = (await provider.connection.getAccountInfo(user2EscrowAta)) ? (await getAccount(provider.connection, user2EscrowAta)).amount : BigInt(0);

    await program.methods
      .releaseEscrow()
      .accounts({
        recipient: user2Keypair.publicKey,
        escrow: escrowPDA,
        mint: escrowMintKeypair.publicKey,
        escrowAuthority: escrowAuthorityPDA,
        escrowTokenAccount: escrowTokenAccount,
        recipientTokenAccount: user2EscrowAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user2Keypair])
      .rpc({ commitment: "confirmed" });

    const user2AtaInfoAfter = await getAccount(provider.connection, user2EscrowAta, "confirmed", TOKEN_PROGRAM_ID)
    const expectedUser2Balance = BigInt(user2InitialBalance.toString()) + BigInt(escrowAmount.toString());
    assert.isTrue(user2AtaInfoAfter.amount === expectedUser2Balance);

    const escrowAccountInfo = await provider.connection.getAccountInfo(escrowPDA);
    assert.isNull(escrowAccountInfo, "Escroe account should be closed")
  });


})

it("9. Tranfer SOL", async () => {
  const amountToTransfer = new BN(1 * LAMPORTS_PER_SOL);
  const user1InitialLamports = await provider.connection.getBalance(user1Keypair.publicKey);
  const user2InitialLamports = await provider.connection.getBalance(user2Keypair.publicKey);

  await program.methods
    .transferSol(amountToTransfer)
    .accounts({
      from: user1Keypair.publicKey,
      to: user2Keypair.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([user1Keypair])
    .rpc({ commitment: "confirmed" })

  const user1AfterLamports = await provider.connection.getBalance(user1Keypair.publicKey);
  const user2AfterLmaports = await provider.connection.getBalance(user2Keypair.publicKey)
})
})