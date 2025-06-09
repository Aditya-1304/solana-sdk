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
    it("should create a multisig with valid parameters", async () => {
      const owners = [owner1.publicKey, owner2.publicKey, owner3.publicKey];
      const threshold = 2;
      const adminThreshold = 3;

      const tx = await program.methods
        .createMultisig(owners, threshold, adminThreshold)
        .accounts({
          creator: creator.publicKey,
          multisig: multisigPda,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([creator])
        .rpc()
      console.log("Multisig created with tx:", tx);

      const multisig = await program.account.multisig.fetch(multisigPda);
      expect(multisig.owners).to.have.lengthOf(3);
      expect(multisig.threshold).to.equal(2);
      expect(multisig.adminThreshold).to.equal(3);
      expect(multisig.transactionCount.toNumber()).to.equal(0);
      expect(multisig.paused).to.be.false;
      expect(multisig.nonce.toNumber()).to.equal(0);

      console.log("✅ Multisig created successfully with proper state!");

    });

    it("Should fail with no owners", async () => {
      const newCreator = Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(newCreator.publicKey, LAMPORTS_PER_SOL),
      );

      const [newMultisigPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("multisig"), newCreator.publicKey.toBuffer()],
        program.programId
      );

      try {
        await program.methods
          .createMultisig([], 1, null)
          .accounts({
            creator: newCreator.publicKey,
            multisig: newMultisigPda,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([newCreator])
          .rpc();
        expect.fail("Should have failed with no owners");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("NoOwners");
        console.log("✅ Correctly rejected empty owners array");

      }
    });

    it("Should fail with invalid threshold", async () => {
      const newCreator = Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(newCreator.publicKey, LAMPORTS_PER_SOL),
        "confirmed"
      );

      const [newMultisigPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("multisig"), newCreator.publicKey.toBuffer()],
        program.programId
      );

      try {
        await program.methods
          .createMultisig([owner1.publicKey], 0, null)
          .accounts({
            creator: newCreator.publicKey,
            multisig: newMultisigPda,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([newCreator])
          .rpc();

        expect.fail("Should have failed with invalid threshold");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("InvalidThreshold");
        console.log("✅ Correctly rejected invalid threshold");
      }
    });

    it("Should fail with duplicate owners", async () => {
      const newCreator = Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(newCreator.publicKey, LAMPORTS_PER_SOL
        ),
        "confirmed"
      );

      const [newMultisigPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("multisig"), newCreator.publicKey.toBuffer()],
        program.programId
      )

      try {
        await program.methods
          .createMultisig([owner1.publicKey, owner1.publicKey], 1, null)
          .accounts({
            creator: newCreator.publicKey,
            multisig: newMultisigPda,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([newCreator])
          .rpc();
        expect.fail("Should have failed with duplicate owners");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("DuplicateOwners");
        console.log("✅ Correctly rejected duplicate owners");
      }
    });

    it("Should fail with too many owners", async () => {
      const newCreator = Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(newCreator.publicKey, LAMPORTS_PER_SOL),
        "confirmed"
      );

      const [newMultisigPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("multisig"), newCreator.publicKey.toBuffer()],
        program.programId
      );

      const tooManyOwners = Array.from({ length: 11 }, () => Keypair.generate().publicKey);

      try {
        await program.methods
          .createMultisig(tooManyOwners, 5, null)
          .accounts({
            creator: newCreator.publicKey,
            multisig: newMultisigPda,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([newCreator])
          .rpc();
        expect.fail("Should have failed with too many owners");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("TooManyOwners");
        console.log("✅ Correctly rejected too many owners");
      }
    })
  });

  describe("2. Transaction Proposal", () => {
    let transactionPda: PublicKey;
    let currentNonce: number;

    before(async () => {
      const multisig = await program.account.multisig.fetch(multisigPda);
      currentNonce = multisig.nonce.toNumber();

      [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );
    });

    it("Should propose transaction succesfully", async () => {
      const tx = await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          {
            transfer: {}
          },
          72
        )
        .accounts({
          proposer: creator.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([creator])
        .rpc();

      console.log("📜 Propose transaction tx:", tx);

      // Verify transaction state
      const transaction = await program.account.transaction.fetch(transactionPda);
      expect(transaction.transactionId.toNumber()).to.equal(0);
      expect(transaction.executed).to.be.false;
      expect(transaction.proposer.toString()).to.equal(creator.publicKey.toString());
      expect(transaction.approvals).to.have.lengthOf(3);
      expect(transaction.approvals.every(approved => !approved)).to.be.true;

      // Verify multisig state updated
      const multisig = await program.account.multisig.fetch(multisigPda);
      expect(multisig.transactionCount.toNumber()).to.equal(1);
      expect(multisig.nonce.toNumber()).to.equal(currentNonce + 1);

      console.log("✅ Transaction proposed successfully!");

    });

    it("Should fail with wrong nonce", async () => {
      try {
        await program.methods
          .proposeTransaction(
            Array.from(testInstruction),
            new anchor.BN(999),
            { transfer: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([owner1])
          .rpc();
        expect.fail("Should have failed with wrong nonce");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("InvalidNonce");
        console.log("✅ Correctly rejected wrong nonce");
      }
    });
    it("Should fail with non-owner proposer", async () => {
      try {
        await program.methods
          .proposeTransaction(
            Array.from(testInstruction),
            new anchor.BN(currentNonce),
            { transfer: {} },
            72
          )
          .accounts({
            proposer: nonOwner.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([nonOwner])
          .rpc();

        expect.fail("Should have failed with non-owner proposer");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("OwnerNotFound");
        console.log("✅ Correctly rejected non-owner proposer");
      }
    });

    it("Should fail with empty instruction data", async () => {
      try {
        await program.methods
          .proposeTransaction(
            [],
            new anchor.BN(currentNonce),
            { transfer: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId
          } as any)
          .signers([owner1])
          .rpc();
        expect.fail("Should have failed with empty instruction data");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("EmptyTransaction");
        console.log("✅ Correctly rejected empty instruction data");
      }
    });

    it("Should fail with oversized instruction data", async () => {
      const oversizedData = Array.from({ length: 1001 }, (_, i) => i % 256); // > 1000 bytes

      try {
        await program.methods
          .proposeTransaction(
            oversizedData,
            new anchor.BN(currentNonce),
            { transfer: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId
          } as any)
          .signers([owner1])
          .rpc();
        expect.fail("Should have failed with oversized instruction data");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("TransactionTooLarge");
        console.log("✅ Correctly rejected oversized instruction data");
      }
    });
  });

  describe("👍 3. Transaction Approval", () => {
    let transactionPda: PublicKey;
    let transactionId: number;

    before(async () => {
      // Create a new transaction to approve
      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();
      transactionId = multisig.transactionCount.toNumber();

      [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          { transfer: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();
    });

    it("✅ Should approve transaction successfully", async () => {
      const tx = await program.methods
        .approveTransaction(new anchor.BN(transactionId))
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner2])
        .rpc();

      console.log("📜 Approve transaction tx:", tx);

      // Verify approval was recorded
      const transaction = await program.account.transaction.fetch(transactionPda);
      expect(transaction.approvals[1]).to.be.true; // owner2 is at index 1

      console.log("✅ Transaction approved successfully!");
    });

    it("❌ Should fail with non-owner approver", async () => {
      try {
        await program.methods
          .approveTransaction(new anchor.BN(transactionId))
          .accounts({
            approver: nonOwner.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .signers([nonOwner])
          .rpc();

        expect.fail("Should have failed with non-owner approver");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("OwnerNotFound");
        console.log("✅ Correctly rejected non-owner approver");
      }
    });

    it("❌ Should fail with double approval", async () => {
      try {
        await program.methods
          .approveTransaction(new anchor.BN(transactionId))
          .accounts({
            approver: owner2.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .signers([owner2])
          .rpc();

        expect.fail("Should have failed with double approval");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("AlreadyApproved");
        console.log("✅ Correctly rejected double approval");
      }
    });

    it("❌ Should fail with wrong transaction ID", async () => {
      try {
        await program.methods
          .approveTransaction(new anchor.BN(999)) // Wrong transaction ID
          .accounts({
            approver: owner3.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .signers([owner3])
          .rpc();

        expect.fail("Should have failed with wrong transaction ID");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("InvalidTransactionId");
        console.log("✅ Correctly rejected wrong transaction ID");
      }
    });
  });

  describe("⚡ 4. Transaction Execution", () => {
    let transactionPda: PublicKey;
    let transactionId: number;

    before(async () => {
      // Create and fully approve a new transaction
      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();
      transactionId = multisig.transactionCount.toNumber();

      [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // Propose transaction
      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          { transfer: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      // Get enough approvals (threshold = 2)
      await program.methods
        .approveTransaction(new anchor.BN(transactionId))
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner2])
        .rpc();

      // Wait a slot to avoid same-slot execution error
      await new Promise(resolve => setTimeout(resolve, 500));
    });

    it("✅ Should execute transaction successfully", async () => {
      const tx = await program.methods
        .executeTransaction(new anchor.BN(transactionId))
        .accounts({
          executor: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner1])
        .rpc();

      console.log("📜 Execute transaction tx:", tx);

      // Verify transaction was executed
      const transaction = await program.account.transaction.fetch(transactionPda);
      expect(transaction.executed).to.be.true;

      console.log("✅ Transaction executed successfully!");
    });

    it("❌ Should fail to execute already executed transaction", async () => {
      try {
        await program.methods
          .executeTransaction(new anchor.BN(transactionId))
          .accounts({
            executor: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed with already executed transaction");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("AlreadyExecuted");
        console.log("✅ Correctly rejected already executed transaction");
      }
    });

    it("❌ Should fail execution with insufficient approvals", async () => {
      // Create a new transaction with only 1 approval (threshold = 2)
      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();
      const newTransactionId = multisig.transactionCount.toNumber();

      const [newTransactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // Propose new transaction
      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          { transfer: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: newTransactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      // Only get 1 approval (need 2)
      await program.methods
        .approveTransaction(new anchor.BN(newTransactionId))
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPda,
          transaction: newTransactionPda,
        })
        .signers([owner2])
        .rpc();

      // Wait a slot
      await new Promise(resolve => setTimeout(resolve, 500));

      try {
        await program.methods
          .executeTransaction(new anchor.BN(newTransactionId))
          .accounts({
            executor: owner1.publicKey,
            multisig: multisigPda,
            transaction: newTransactionPda,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed with insufficient approvals");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("NotEnoughApprovals");
        console.log("✅ Correctly rejected insufficient approvals");
      }
    });

    it("❌ Should fail execution by non-owner", async () => {
      // Create another transaction for this test
      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();
      const newTransactionId = multisig.transactionCount.toNumber();

      const [newTransactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // Propose and fully approve transaction
      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          { transfer: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: newTransactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      await program.methods
        .approveTransaction(new anchor.BN(newTransactionId))
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPda,
          transaction: newTransactionPda,
        })
        .signers([owner2])
        .rpc();

      // Wait a slot
      await new Promise(resolve => setTimeout(resolve, 500));

      try {
        await program.methods
          .executeTransaction(new anchor.BN(newTransactionId))
          .accounts({
            executor: nonOwner.publicKey,
            multisig: multisigPda,
            transaction: newTransactionPda,
          })
          .signers([nonOwner])
          .rpc();

        expect.fail("Should have failed with non-owner executor");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("OwnerNotFound");
        console.log("✅ Correctly rejected non-owner executor");
      }
    });
  });

  describe("🚨 5. Emergency Controls", () => {
    it("✅ Should pause multisig successfully", async () => {
      const tx = await program.methods
        .emergencyPause()
        .accounts({
          caller: owner1.publicKey,
          multisig: multisigPda,
        })
        .signers([owner1])
        .rpc();

      console.log("📜 Emergency pause tx:", tx);

      // Verify paused state
      const multisig = await program.account.multisig.fetch(multisigPda);
      expect(multisig.paused).to.be.true;
      expect(multisig.pausedBy.toString()).to.equal(owner1.publicKey.toString());

      console.log("✅ Multisig paused successfully!");
    });

    it("❌ Should fail to propose when paused", async () => {
      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();

      const [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      try {
        await program.methods
          .proposeTransaction(
            Array.from(testInstruction),
            new anchor.BN(currentNonce),
            { transfer: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed when multisig is paused");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("MultisigPaused");
        console.log("✅ Correctly rejected transaction when paused");
      }
    });

    it("✅ Should unpause multisig with admin approval", async () => {
      // Create unpause transaction
      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();
      const transactionId = multisig.transactionCount.toNumber();

      const [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // Create the unpause transaction (admin action)
      await program.methods
        .proposeTransaction(
          Array.from(Buffer.from("unpause")),
          new anchor.BN(currentNonce),
          { adminAction: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      // Get all 3 admin approvals (admin_threshold = 3)
      await program.methods
        .approveTransaction(new anchor.BN(transactionId))
        .accounts({
          approver: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner1])
        .rpc();

      await program.methods
        .approveTransaction(new anchor.BN(transactionId))
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner2])
        .rpc();

      await program.methods
        .approveTransaction(new anchor.BN(transactionId))
        .accounts({
          approver: owner3.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner3])
        .rpc();

      // Now unpause
      const tx = await program.methods
        .unpause(new anchor.BN(transactionId))
        .accounts({
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .rpc();

      console.log("📜 Unpause tx:", tx);

      // Verify unpaused state
      const updatedMultisig = await program.account.multisig.fetch(multisigPda);
      expect(updatedMultisig.paused).to.be.false;
      expect(updatedMultisig.pausedBy.toString()).to.equal(SystemProgram.programId.toString());

      console.log("✅ Multisig unpaused successfully!");
    });

    it("❌ Should fail emergency pause by non-owner", async () => {
      try {
        await program.methods
          .emergencyPause()
          .accounts({
            caller: nonOwner.publicKey,
            multisig: multisigPda,
          })
          .signers([nonOwner])
          .rpc();

        expect.fail("Should have failed with non-owner caller");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("OwnerNotFound");
        console.log("✅ Correctly rejected non-owner emergency pause");
      }
    });
  });

  describe("👥 6. Admin Functions", () => {
    describe("🔧 Change Threshold", () => {
      it("✅ Should change threshold with admin approval", async () => {
        // Create change threshold transaction
        const multisig = await program.account.multisig.fetch(multisigPda);
        const currentNonce = multisig.nonce.toNumber();
        const transactionId = multisig.transactionCount.toNumber();

        const [transactionPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("transaction"),
            multisigPda.toBuffer(),
            Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
          ],
          program.programId
        );

        // Propose change threshold transaction
        await program.methods
          .proposeTransaction(
            Array.from(Buffer.from("change_threshold")),
            new anchor.BN(currentNonce),
            { adminAction: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        // Get all 3 admin approvals
        for (const owner of [owner1, owner2, owner3]) {
          await program.methods
            .approveTransaction(new anchor.BN(transactionId))
            .accounts({
              approver: owner.publicKey,
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .signers([owner])
            .rpc();
        }

        // Change threshold to 3
        const tx = await program.methods
          .changeThreshold(new anchor.BN(transactionId), 3)
          .accounts({
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .rpc();

        console.log("📜 Change threshold tx:", tx);

        // Verify threshold changed
        const updatedMultisig = await program.account.multisig.fetch(multisigPda);
        expect(updatedMultisig.threshold).to.equal(3);

        console.log("✅ Threshold changed successfully!");
      });

      it("❌ Should fail change threshold with invalid value", async () => {
        // Create change threshold transaction
        const multisig = await program.account.multisig.fetch(multisigPda);
        const currentNonce = multisig.nonce.toNumber();
        const transactionId = multisig.transactionCount.toNumber();

        const [transactionPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("transaction"),
            multisigPda.toBuffer(),
            Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
          ],
          program.programId
        );

        // Propose change threshold transaction
        await program.methods
          .proposeTransaction(
            Array.from(Buffer.from("change_threshold")),
            new anchor.BN(currentNonce),
            { adminAction: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        // Get all 3 admin approvals
        for (const owner of [owner1, owner2, owner3]) {
          await program.methods
            .approveTransaction(new anchor.BN(transactionId))
            .accounts({
              approver: owner.publicKey,
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .signers([owner])
            .rpc();
        }

        try {
          await program.methods
            .changeThreshold(new anchor.BN(transactionId), 0) // Invalid threshold
            .accounts({
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .rpc();

          expect.fail("Should have failed with invalid threshold");
        } catch (error) {
          expect(error.error.errorCode.code).to.equal("InvalidThreshold");
          console.log("✅ Correctly rejected invalid threshold");
        }
      });
    });

    describe("➕ Add Owner", () => {
      it("✅ Should add owner with admin approval", async () => {
        // Create add owner transaction
        const multisig = await program.account.multisig.fetch(multisigPda);
        const currentNonce = multisig.nonce.toNumber();
        const transactionId = multisig.transactionCount.toNumber();

        const [transactionPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("transaction"),
            multisigPda.toBuffer(),
            Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
          ],
          program.programId
        );

        // Propose add owner transaction
        await program.methods
          .proposeTransaction(
            Array.from(Buffer.from("add_owner")),
            new anchor.BN(currentNonce),
            { adminAction: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        // Get all 3 admin approvals
        for (const owner of [owner1, owner2, owner3]) {
          await program.methods
            .approveTransaction(new anchor.BN(transactionId))
            .accounts({
              approver: owner.publicKey,
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .signers([owner])
            .rpc();
        }

        // Add new owner
        const tx = await program.methods
          .addOwner(new anchor.BN(transactionId), owner4.publicKey)
          .accounts({
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .rpc();

        console.log("📜 Add owner tx:", tx);

        // Verify owner added
        const updatedMultisig = await program.account.multisig.fetch(multisigPda);
        expect(updatedMultisig.owners).to.have.lengthOf(4);
        expect(updatedMultisig.owners.map(o => o.toString())).to.include(owner4.publicKey.toString());

        console.log("✅ Owner added successfully!");
      });

      it("❌ Should fail to add duplicate owner", async () => {
        // Create add owner transaction for existing owner
        const multisig = await program.account.multisig.fetch(multisigPda);
        const currentNonce = multisig.nonce.toNumber();
        const transactionId = multisig.transactionCount.toNumber();

        const [transactionPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("transaction"),
            multisigPda.toBuffer(),
            Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
          ],
          program.programId
        );

        // Propose add owner transaction
        await program.methods
          .proposeTransaction(
            Array.from(Buffer.from("add_owner")),
            new anchor.BN(currentNonce),
            { adminAction: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        // Get all 3 admin approvals
        for (const owner of [owner1, owner2, owner3]) {
          await program.methods
            .approveTransaction(new anchor.BN(transactionId))
            .accounts({
              approver: owner.publicKey,
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .signers([owner])
            .rpc();
        }

        try {
          await program.methods
            .addOwner(new anchor.BN(transactionId), owner1.publicKey) // Duplicate owner
            .accounts({
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .rpc();

          expect.fail("Should have failed with duplicate owner");
        } catch (error) {
          expect(error.error.errorCode.code).to.equal("DuplicateOwners");
          console.log("✅ Correctly rejected duplicate owner");
        }
      });
    });

    describe("➖ Remove Owner", () => {
      it("✅ Should remove owner with admin approval", async () => {
        // Create remove owner transaction
        const multisig = await program.account.multisig.fetch(multisigPda);
        const currentNonce = multisig.nonce.toNumber();
        const transactionId = multisig.transactionCount.toNumber();

        const [transactionPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("transaction"),
            multisigPda.toBuffer(),
            Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
          ],
          program.programId
        );

        // Propose remove owner transaction
        await program.methods
          .proposeTransaction(
            Array.from(Buffer.from("remove_owner")),
            new anchor.BN(currentNonce),
            { adminAction: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        // Get all 3 admin approvals (current admin threshold)
        for (const owner of [owner1, owner2, owner3]) {
          await program.methods
            .approveTransaction(new anchor.BN(transactionId))
            .accounts({
              approver: owner.publicKey,
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .signers([owner])
            .rpc();
        }

        // Remove owner4 (recently added)
        const tx = await program.methods
          .removeOwner(new anchor.BN(transactionId), owner4.publicKey)
          .accounts({
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .rpc();

        console.log("📜 Remove owner tx:", tx);

        // Verify owner removed
        const updatedMultisig = await program.account.multisig.fetch(multisigPda);
        expect(updatedMultisig.owners).to.have.lengthOf(3);
        expect(updatedMultisig.owners.map(o => o.toString())).to.not.include(owner4.publicKey.toString());

        console.log("✅ Owner removed successfully!");
      });

      it("❌ Should fail to remove non-existent owner", async () => {
        // Create remove owner transaction
        const multisig = await program.account.multisig.fetch(multisigPda);
        const currentNonce = multisig.nonce.toNumber();
        const transactionId = multisig.transactionCount.toNumber();

        const [transactionPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("transaction"),
            multisigPda.toBuffer(),
            Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
          ],
          program.programId
        );

        // Propose remove owner transaction
        await program.methods
          .proposeTransaction(
            Array.from(Buffer.from("remove_owner")),
            new anchor.BN(currentNonce),
            { adminAction: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        // Get all 3 admin approvals
        for (const owner of [owner1, owner2, owner3]) {
          await program.methods
            .approveTransaction(new anchor.BN(transactionId))
            .accounts({
              approver: owner.publicKey,
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .signers([owner])
            .rpc();
        }

        try {
          await program.methods
            .removeOwner(new anchor.BN(transactionId), nonOwner.publicKey) // Non-existent owner
            .accounts({
              multisig: multisigPda,
              transaction: transactionPda,
            })
            .rpc();

          expect.fail("Should have failed with non-existent owner");
        } catch (error) {
          expect(error.error.errorCode.code).to.equal("OwnerNotFound");
          console.log("✅ Correctly rejected non-existent owner removal");
        }
      });
    });
  });

  describe("⏰ 7. Rate Limiting & DOS Protection", () => {
    it("❌ Should fail rapid transaction proposals", async () => {
      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();

      const [transactionPda1] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // First proposal should succeed
      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          { transfer: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda1,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      // Immediate second proposal should fail (rate limit)
      const updatedMultisig = await program.account.multisig.fetch(multisigPda);
      const newNonce = updatedMultisig.nonce.toNumber();

      const [transactionPda2] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(updatedMultisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      try {
        await program.methods
          .proposeTransaction(
            Array.from(testInstruction),
            new anchor.BN(newNonce),
            { transfer: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda2,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed due to rate limiting");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("RateLimitExceeded");
        console.log("✅ Rate limiting working correctly!");
      }
    });

    it("❌ Should fail with overly complex transaction", async () => {
      // Wait for rate limit to pass
      await new Promise(resolve => setTimeout(resolve, 2000));

      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();

      const [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // Create artificially complex instruction data
      const complexData = Array.from({ length: 500 }, (_, i) => {
        // Create patterns that will increase complexity score
        if (i % 4 === 0) return (1000000 >> 0) & 0xFF;
        if (i % 4 === 1) return (1000000 >> 8) & 0xFF;
        if (i % 4 === 2) return (1000000 >> 16) & 0xFF;
        if (i % 4 === 3) return (1000000 >> 24) & 0xFF;
        return i % 256;
      });

      try {
        await program.methods
          .proposeTransaction(
            complexData,
            new anchor.BN(currentNonce),
            { transfer: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed due to transaction complexity");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("TransactionTooComplex");
        console.log("✅ Complexity limiting working correctly!");
      }
    });
  });

  describe("⏳ 8. Transaction Expiration", () => {
    it("❌ Should fail to execute expired transaction", async () => {
      // Wait for rate limit to pass
      await new Promise(resolve => setTimeout(resolve, 2000));

      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();
      const transactionId = multisig.transactionCount.toNumber();

      const [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // Create transaction with very short expiration (1 hour)
      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          { transfer: {} },
          0 // expires immediately (0 hours)
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      // Get enough approvals
      await program.methods
        .approveTransaction(new anchor.BN(transactionId))
        .accounts({
          approver: owner2.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner2])
        .rpc();

      await program.methods
        .approveTransaction(new anchor.BN(transactionId))
        .accounts({
          approver: owner3.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
        })
        .signers([owner3])
        .rpc();

      // Wait a bit for expiration
      await new Promise(resolve => setTimeout(resolve, 1000));

      try {
        await program.methods
          .executeTransaction(new anchor.BN(transactionId))
          .accounts({
            executor: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed with expired transaction");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("TransactionExpired");
        console.log("✅ Transaction expiration working correctly!");
      }
    });
  });

  describe("📊 9. State Validation & Integrity", () => {
    it("✅ Should maintain consistent state throughout operations", async () => {
      const multisig = await program.account.multisig.fetch(multisigPda);

      // Verify state integrity
      expect(multisig.owners).to.have.lengthOf(3);
      expect(multisig.threshold).to.equal(3);
      expect(multisig.adminThreshold).to.equal(3);
      expect(multisig.paused).to.be.false;
      expect(multisig.transactionCount.toNumber()).to.be.greaterThan(0);
      expect(multisig.nonce.toNumber()).to.be.greaterThan(0);

      // Verify no duplicate owners
      const ownerStrings = multisig.owners.map(o => o.toString());
      const uniqueOwners = [...new Set(ownerStrings)];
      expect(uniqueOwners).to.have.lengthOf(ownerStrings.length);

      // Verify thresholds are valid
      expect(multisig.threshold).to.be.at.most(multisig.owners.length);
      expect(multisig.adminThreshold).to.be.at.most(multisig.owners.length);
      expect(multisig.adminThreshold).to.be.at.least(multisig.threshold);

      console.log("✅ State integrity verified!");
    });

    it("✅ Should prevent same-slot execution", async () => {
      // Wait for rate limit
      await new Promise(resolve => setTimeout(resolve, 2000));

      const multisig = await program.account.multisig.fetch(multisigPda);
      const currentNonce = multisig.nonce.toNumber();
      const transactionId = multisig.transactionCount.toNumber();

      const [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      // Propose transaction
      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(currentNonce),
          { transfer: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      // Get all required approvals
      for (const owner of [owner1, owner2, owner3]) {
        await program.methods
          .approveTransaction(new anchor.BN(transactionId))
          .accounts({
            approver: owner.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .signers([owner])
          .rpc();
      }

      // Try to execute immediately (same slot) - should fail
      try {
        await program.methods
          .executeTransaction(new anchor.BN(transactionId))
          .accounts({
            executor: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed due to same-slot execution");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("SameSlotExecution");
        console.log("✅ Same-slot execution protection working!");
      }
    });
  });

  describe("🔒 10. Nonce & Replay Protection", () => {
    it("❌ Should prevent nonce reuse", async () => {
      const multisig = await program.account.multisig.fetch(multisigPda);
      const oldNonce = multisig.nonce.toNumber() - 1; // Use old nonce

      const [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisig.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      try {
        await program.methods
          .proposeTransaction(
            Array.from(testInstruction),
            new anchor.BN(oldNonce), // Reusing old nonce
            { transfer: {} },
            72
          )
          .accounts({
            proposer: owner1.publicKey,
            multisig: multisigPda,
            transaction: transactionPda,
            systemProgram: SystemProgram.programId,
          })
          .signers([owner1])
          .rpc();

        expect.fail("Should have failed with reused nonce");
      } catch (error) {
        expect(error.error.errorCode.code).to.equal("InvalidNonce");
        console.log("✅ Nonce replay protection working!");
      }
    });

    it("✅ Should increment nonce correctly", async () => {
      const multisigBefore = await program.account.multisig.fetch(multisigPda);
      const nonceBefore = multisigBefore.nonce.toNumber();

      // Wait for rate limit
      await new Promise(resolve => setTimeout(resolve, 2000));

      const [transactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("transaction"),
          multisigPda.toBuffer(),
          Buffer.from(multisigBefore.transactionCount.toArrayLike(Buffer, "le", 8))
        ],
        program.programId
      );

      await program.methods
        .proposeTransaction(
          Array.from(testInstruction),
          new anchor.BN(nonceBefore),
          { transfer: {} },
          72
        )
        .accounts({
          proposer: owner1.publicKey,
          multisig: multisigPda,
          transaction: transactionPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();

      const multisigAfter = await program.account.multisig.fetch(multisigPda);
      const nonceAfter = multisigAfter.nonce.toNumber();

      expect(nonceAfter).to.equal(nonceBefore + 1);
      console.log("✅ Nonce incremented correctly!");
    });
  });

})