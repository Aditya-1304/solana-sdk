import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MultisigModule } from "../target/types/multisig_module";
import { Keypair, PublicKey } from "@solana/web3.js";
import { assert, expect } from "chai";

describe("multisig", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MultisigModule as Program<MultisigModule>;

  let creator: Keypair;
  let owner1: Keypair;
  let owner2: Keypair;
  let owner3: Keypair;

  beforeEach(() => {
    creator = Keypair.generate();
    owner1 = Keypair.generate();
    owner2 = Keypair.generate();
    owner3 = Keypair.generate();
  })

  const expectAnchorError = async (fn: Promise<any>, expectedErrorMessage: string) => {
    try {
      await fn;
      assert.fail("Expected promise to be rejected but it resolved successfully");
    } catch (error: any) {
      const errorStr = error.toString();
      expect(errorStr).to.include(expectedErrorMessage);
    }
  }

  const findMultisigPDA = (creatorPubkey: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("multisig"), creatorPubkey.toBuffer()],
      program.programId
    );
  };

  const fundAccount = async (keypair: Keypair, lamports: number = 1000000000) => {
    const signature = await provider.connection.requestAirdrop(
      keypair.publicKey,
      lamports
    );
    await provider.connection.confirmTransaction(signature);
  }

  describe("Create Multisig", () => {
    it("Should create a 2-of-3 multisig successfully", async () => {
      await fundAccount(creator);

      const owners = [owner1.publicKey, owner2.publicKey, owner3.publicKey];
      const threshold = 2;

      const [multisigPDA] = findMultisigPDA(creator.publicKey);

      await program.methods
        .createMultisig(owners, threshold)
        .accounts({
          creator: creator.publicKey,
          multisig: multisigPDA,
          systemProgram: anchor.web3.SystemProgram.programId
        } as any)
        .signers([creator])
        .rpc();

      const multisigAccount = await program.account.multisig.fetch(multisigPDA);

      assert.equal(multisigAccount.threshold, threshold);
      assert.equal(multisigAccount.owners.length, 3)
      assert.equal(multisigAccount.transactionCount.toNumber(), 0);

      owners.forEach((owner, index) => {
        assert.isTrue(multisigAccount.owners[index].equals(owner));
      });

      console.log("✅ Multisig created successfully!");
      console.log("   Multisig PDA:", multisigPDA.toString());
      console.log("   Owners:", multisigAccount.owners.map(o => o.toString()));
      console.log("   Threshold:", multisigAccount.threshold);
    });

    it("Should fail with invalid threshold (0)", async () => {
      await fundAccount(creator);

      const owners = [owner1.publicKey, owner2.publicKey];
      const threshold = 0;

      const [multisigPDA] = findMultisigPDA(creator.publicKey);

      await expectAnchorError(
        program.methods
          .createMultisig(owners, threshold)
          .accounts({
            creator: creator.publicKey, // ← Added this
            multisig: multisigPDA,
            systemProgram: anchor.web3.SystemProgram.programId,
          } as any)
          .signers([creator])
          .rpc(),
        "Invalid threshold" // This should now work
      );

      console.log("✅ Correctly rejected threshold = 0");
    });

    it("Should fail with threshold > number of owners", async () => {
      await fundAccount(creator)

      const owners = [owner1.publicKey, owner2.publicKey]
      const threshold = 3;

      const [multisigPDA] = findMultisigPDA(creator.publicKey);

      try {
        await program.methods
          .createMultisig(owners, threshold)
          .accounts({
            creator: creator.publicKey,
            multisig: multisigPDA,
            systemProgram: anchor.web3.SystemProgram.programId,
          } as any)
          .signers([creator])
          .rpc();

        assert.fail("Should have thrown an error");
      } catch (error) {
        expect(error.message).to.include("Invalid threshold");
        console.log("✅ Correctly rejected threshold > owners");
      }
    });

    it("Should fail with duplicate owners", async () => {
      await fundAccount(creator);

      const owners = [owner1.publicKey, owner1.publicKey];
      const threshold = 1;

      const [multisigPDA] = findMultisigPDA(creator.publicKey);

      await expectAnchorError(
        program.methods
          .createMultisig(owners, threshold)
          .accounts({
            creator: creator.publicKey,
            multisig: multisigPDA,
            systemProgram: anchor.web3.SystemProgram.programId,
          } as any)
          .signers([creator])
          .rpc(),
        "DuplicateOwners" // Try without the space
      );

      console.log("✅ Correctly rejected duplicate owners");
    });

    it("Should create a 1-of-1 multisig (single owner)", async () => {
      await fundAccount(creator)

      const owners = [owner1.publicKey]
      const threshold = 1;

      const [multisigPDA] = findMultisigPDA(creator.publicKey);

      await program.methods
        .createMultisig(owners, threshold)
        .accounts({
          creator: creator.publicKey,
          multisig: multisigPDA,
          systemProgram: anchor.web3.SystemProgram.programId
        } as any)
        .signers([creator])
        .rpc()

      const multisigAccount = await program.account.multisig.fetch(multisigPDA);

      assert.equal(multisigAccount.threshold, 1);
      assert.equal(multisigAccount.owners.length, 1);
      assert.isTrue(multisigAccount.owners[0].equals(owner1.publicKey));

      console.log("✅ 1-of-1 multisig created successfully!");

    })
  })
})
