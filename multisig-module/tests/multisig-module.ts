import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MultisigModule } from "../target/types/multisig_module";
import {
  Keypair,
  SystemProgram,
  PublicKey,
  LAMPORTS_PER_SOL,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { expect } from "chai";

describe("Multisig Module - Production Test Suite", () => {
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.MultisigModule as Program<MultisigModule>;

  const creator = Keypair.generate();
  const owner1 = Keypair.generate();
  const owner2 = Keypair.generate();
  const owner3 = Keypair.generate();
  const owner4 = Keypair.generate();
  const owner5 = Keypair.generate();
  const nonOwner = Keypair.generate();

  let multisigPda: PublicKey;
  let multisigBump: number;

  const testInstruction = Buffer.from("test instruction data");
  const testSeed = Array.from({ length: 32 }, (_, i) => i);

  before(async () => {
    console.log("Setting up test environment...");

    const accounts = [creator, owner1, owner2, owner3, owner4, owner5, nonOwner];
    for (const account of accounts) {
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(account.publicKey, 2 * LAMPORTS_PER_SOL),
        "confirmed"
      );
    }

    [multisigPda, multisigBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("multisig"), creator.publicKey.toBuffer()],
      program.programId
    );

    console.log("✅ Test environment ready!");
    console.log("📍 Creator:", creator.publicKey.toString());
    console.log("📍 Multisig PDA:", multisigPda.toString());

  });


  describe("1. Multisig Creation", () => {

  })
})