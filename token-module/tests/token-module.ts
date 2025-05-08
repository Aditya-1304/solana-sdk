import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { TokenModule } from '../target/types/token_module';
import { assert, expect } from 'chai';
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram } from '@solana/web3.js';
import { createMint, createAccount, mintTo, getAccount } from '@solana/spl-token';

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



})