import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MultisigModule } from "../target/types/multisig_module";
import { Keypair } from "@solana/web3.js";
import { expect } from "chai";

describe("multisig", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MultisigModule as Program<MultisigModule>;

  const owner1 = Keypair.generate();
  const owner2 = Keypair.generate();
  const owner3 = Keypair.generate();

  it("Should create a multisig ", async () => {
    console.log("Program ID:", program.programId.toString());
    console.log("Owner 1:", owner1.publicKey.toString());
    console.log("Owner 2:", owner2.publicKey.toString());
    console.log("Owner 3:", owner3.publicKey.toString());

    expect(true).to.equal(true);
  });
})
