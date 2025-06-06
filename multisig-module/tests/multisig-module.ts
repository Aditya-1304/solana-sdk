import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
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


  describe("Propose Transaction", () => {
    let multisigPDA: PublicKey;
    let transactionPDA: PublicKey;

    beforeEach(async () => {
      await fundAccount(creator);
      const owners = [owner1.publicKey, owner2.publicKey, owner3.publicKey];
      const threshold = 2;

      [multisigPDA] = findMultisigPDA(creator.publicKey);

      await program.methods
        .createMultisig(owners, threshold)
        .accounts({
          creator: creator.publicKey,
          multisig: multisigPDA,
          systemProgram: anchor.web3.SystemProgram.programId
        } as any)
        .signers([creator])
        .rpc();
    });

    const findTransactionPDA = (multisigPDA: PublicKey, transactionId: number) => {
      // Create the buffer for the transaction ID (u64 little-endian)
      const buffer = Buffer.allocUnsafe(8);
      buffer.writeBigUInt64LE(BigInt(transactionId), 0);

      return PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPDA.toBuffer(),
          buffer, // ← Now 'buffer' is properly defined
        ],
        program.programId
      );
    };

    it("Should propose a transaction successfully", async () => {
      await fundAccount(owner1);

      const instructionData = Buffer.from("dummy_instruction_data");
      [transactionPDA] = findTransactionPDA(multisigPDA, 0);

      await program.methods
        .proposeTransaction(instructionData) // ← Keep as Buffer
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPDA,
          transaction: transactionPDA,
          systemProgram: anchor.web3.SystemProgram.programId,
        } as any)
        .signers([owner1])
        .rpc();

      const transactionAccount = await program.account.transaction.fetch(transactionPDA);

      assert.isTrue(transactionAccount.multisig.equals(multisigPDA));
      assert.isTrue(transactionAccount.proposer.equals(owner1.publicKey));
      assert.equal(transactionAccount.transactionId.toNumber(), 0);
      assert.isFalse(transactionAccount.executed);
      assert.equal(transactionAccount.approvals.length, 3);
      // assert.deepEqual(transactionAccount.instructionData, Array.from(instructionData)); // ← Compare with Array.from()

      console.log("✅ Transaction proposed successfully!");
      console.log("   Transaction ID:", transactionAccount.transactionId.toString());
      console.log("   Proposer:", transactionAccount.proposer.toString());
    });

    it("Should fail when non-owner tries to propose", async () => {
      const nonOwner = Keypair.generate();
      await fundAccount(nonOwner);

      const instructionData = Buffer.from("dummy_instruction_data");
      [transactionPDA] = findTransactionPDA(multisigPDA, 0);

      await expectAnchorError(
        program.methods
          .proposeTransaction(instructionData) // ← Keep as Buffer
          .accounts({
            proposer: nonOwner.publicKey,
            multisig: multisigPDA,
            transaction: transactionPDA,
            systemProgram: anchor.web3.SystemProgram.programId,
          } as any)
          .signers([nonOwner])
          .rpc(),
        "Owner not found"
      );
      console.log("✅ Correctly rejected non-owner proposal");
    });

    it("Should fail when empty instruction data", async () => {
      await fundAccount(owner1);

      const instructionData = Buffer.from([]); // Empty buffer
      [transactionPDA] = findTransactionPDA(multisigPDA, 0);

      await expectAnchorError(
        program.methods
          .proposeTransaction(instructionData) // ← Keep as Buffer
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPDA,
            transaction: transactionPDA,
            systemProgram: anchor.web3.SystemProgram.programId,
          } as any)
          .signers([owner1])
          .rpc(),
        "Empty transaction"
      );
      console.log("✅ Correctly rejected empty instruction data");
    });
  })

  describe("Approve Transaction", () => {
    let multisigPDA: PublicKey;
    let transactionPDA: PublicKey;
    const transactionId = new BN(0);
    beforeEach(async () => {

      await fundAccount(creator);
      const owners = [owner1.publicKey, owner2.publicKey, owner3.publicKey];
      const threshold = 2;

      [multisigPDA] = findMultisigPDA(creator.publicKey);

      await program.methods
        .createMultisig(owners, threshold)
        .accounts({
          creator: creator.publicKey,
          multisig: multisigPDA,
          systemProgram: anchor.web3.SystemProgram.programId
        } as any)
        .signers([creator])
        .rpc();

      await fundAccount(owner1);
      const instructionData = Buffer.from("dummy_instruction_data");
      [transactionPDA] = findTransactionPDA(multisigPDA, 0);

      await program.methods
        .proposeTransaction(instructionData)
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPDA,
          transaction: transactionPDA,
          systemProgram: anchor.web3.SystemProgram.programId,
        } as any)
        .signers([owner1])
        .rpc();
    });

    const findTransactionPDA = (multisigPDA: PublicKey, transactionId: number) => {
      const buffer = Buffer.allocUnsafe(8);
      buffer.writeBigUInt64LE(BigInt(transactionId), 0);

      return PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPDA.toBuffer(),
          buffer,
        ],
        program.programId
      );
    };

    it("Should approve a transaction successfully", async () => {
      await fundAccount(owner2);

      await program.methods
        .approveTransaction(transactionId)
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPDA,
          transaction: transactionPDA,
        } as any)
        .signers([owner2])
        .rpc();

      const transactionAccount = await program.account.transaction.fetch(transactionPDA);

      assert.equal(transactionAccount.approvals.length, 3);
      assert.isFalse(transactionAccount.approvals[0]); // owner1 index
      assert.isTrue(transactionAccount.approvals[1]);  // owner2 index (approved)
      assert.isFalse(transactionAccount.approvals[2]); // owner3 index

      console.log("✅ Transaction approved successfully!");
      console.log("   Approvals:", transactionAccount.approvals);

    });

    it("Should allow multiple approvals", async () => {
      await fundAccount(owner2);
      await fundAccount(owner3);

      await program.methods
        .approveTransaction(transactionId)
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPDA,
          transaction: transactionPDA,
        } as any)
        .signers([owner2])
        .rpc();

      await program.methods
        .approveTransaction(transactionId)
        .accounts({
          approver: owner3.publicKey,
          multisig: multisigPDA,
          transaction: transactionPDA,
        } as any)
        .signers([owner3])
        .rpc();

      const transactionAccount = await program.account.transaction.fetch(transactionPDA);

      assert.isFalse(transactionAccount.approvals[0]);
      assert.isTrue(transactionAccount.approvals[1]);
      assert.isTrue(transactionAccount.approvals[2]);

      const approvalCount = transactionAccount.approvals.filter(approved => approved).length;
      assert.equal(approvalCount, 2);

      console.log("✅ Multiple approvals recorded successfully!");
      console.log("   Approvals:", transactionAccount.approvals);
    });

    it("Should fail when non-owner tries to approve", async () => {
      const nonOwner = Keypair.generate();
      await fundAccount(nonOwner);

      await expectAnchorError(
        program.methods
          .approveTransaction(transactionId)
          .accounts({
            approver: nonOwner.publicKey,
            multisig: multisigPDA,
            transaction: transactionPDA,
          } as any)
          .signers([nonOwner])
          .rpc(),
        "Owner not found"
      );

      console.log("✅ Correctly rejected non-owner approval");

    });

    it("Should fail when already approved", async () => {
      await fundAccount(owner2);

      await program.methods
        .approveTransaction(transactionId)
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPDA,
          transaction: transactionPDA,
        } as any)
        .signers([owner2])
        .rpc();

      await expectAnchorError(
        program.methods
          .approveTransaction(transactionId)
          .accounts({
            approver: owner2.publicKey,
            multisig: multisigPDA,
            transaction: transactionPDA,
          } as any)
          .signers([owner2])
          .rpc(),
        "Already approved"
      );

      console.log("✅ Correctly rejected already approved transaction");
    });

    it("Should fail with invalid transaction ID", async () => {
      await fundAccount(owner2);
      const invalidTransactionId = new BN(999);

      const [invalidTransactionPDA] = findTransactionPDA(multisigPDA, 999);

      await expectAnchorError(
        program.methods
          .approveTransaction(invalidTransactionId)
          .accounts({
            approver: owner2.publicKey,
            multisig: multisigPDA,
            transaction: invalidTransactionPDA,
          } as any)
          .signers([owner2])
          .rpc(),
        "AccountNotInitialized" // Account doesn't exist
      );

      console.log("✅ Correctly rejected invalid transaction ID");
    });


  })
})
