import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { TokenModule } from "../target/types/token_module";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Transaction,
} from "@solana/web3.js";
import { assert } from "chai";

describe("token-module", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenModule as Program<TokenModule>;
  const wallet = provider.wallet as anchor.Wallet;

  const adminKeypair = Keypair.generate();
  const userKeypair = Keypair.generate();
  const mintKeypair = Keypair.generate();

  let tokenAuthorityPDA: PublicKey;
  let tokenAuthorityBump: number;
  let tokenMetadataPDA: PublicKey;
  let tokenMetadataBump: number;

  const tokenName = "Test Token";
  const tokenSymbol = "TEST";
  const tokenDecimals = 9;
  const tokenUri = "https://test.com/metadata.json";
  // Fix: Use string constructor for large numbers
  const tokenMaxSupply = new BN("1000000000000000000"); // 1 billion tokens with 9 decimals

  before(async () => {
    const adminAirdrop = await provider.connection.requestAirdrop(
      adminKeypair.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    const userAirdrop = await provider.connection.requestAirdrop(
      userKeypair.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    await provider.connection.confirmTransaction(adminAirdrop);
    await provider.connection.confirmTransaction(userAirdrop);

    [tokenAuthorityPDA, tokenAuthorityBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("token_authority")],
      program.programId
    );

    [tokenMetadataPDA, tokenMetadataBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("token_metadata"), mintKeypair.publicKey.toBuffer()],
      program.programId,
    );
  });

  it("Initialize token authority", async () => {
    await program.methods
      .initializeTokenAuthority()
      .accounts({
        admin: adminKeypair.publicKey,
        tokenAuthority: tokenAuthorityPDA,
        systemProgram: SystemProgram.programId, // Fix: Use .programId here
      } as any)
      .signers([adminKeypair])
      .rpc();

    const tokenAuthority = await program.account.tokenAuthority.fetch(tokenAuthorityPDA);
    assert.equal(tokenAuthority.admin.toString(), adminKeypair.publicKey.toString());
    assert.equal(tokenAuthority.bump, tokenAuthorityBump);
  });
});

