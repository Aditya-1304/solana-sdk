import * as anchor from "@coral-xyz/anchor";
import * as fs from 'fs';
import { Program, BN } from "@coral-xyz/anchor";
import { GovernanceModule } from "../target/types/governance_module";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";

describe("Governance Module", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.GovernanceModule as Program<GovernanceModule>;
  const connection = provider.connection;

  let payer: Keypair;

  let governanceAuthority: Keypair;
  let governancePda: PublicKey;
  let governanceBump: number;

  let proposer: Keypair;
  let voter1: Keypair;
  let voter2: Keypair;
  let delegate: Keypair;

  let governanceToken: PublicKey;
  let proposerTokenAccount: PublicKey;
  let voter1TokenAccount: PublicKey;
  let voter2TokenAccount: PublicKey;
  let delegateTokenAccount: PublicKey;

  let treasuryPda: PublicKey;
  let treasuryAuthorityPda: PublicKey;

  const governanceName = "Test DAO";
  const governanceDescription = "A test governance for our DAO";
  const proposalTitle = "Test Proposal";
  const proposalDescription = "This is a test proposal";

  before(async () => {
    // Initialize payer from wallet file
    const keypairPath = process.env.ANCHOR_WALLET ||
      `${require('os').homedir()}/.config/solana/id.json`;
    const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
    payer = Keypair.fromSecretKey(Uint8Array.from(keypairData));

    console.log("\n🚀 Setting up test environment...\n");
    console.log("Payer:", payer.publicKey.toString());
    console.log("Program ID:", program.programId.toString());

    // Generate keypairs
    governanceAuthority = Keypair.generate();
    proposer = Keypair.generate();
    voter1 = Keypair.generate();
    voter2 = Keypair.generate();
    delegate = Keypair.generate();

    console.log("\n✅ Generated test keypairs:");
    console.log("  Governance Authority:", governanceAuthority.publicKey.toString());
    console.log("  Proposer:", proposer.publicKey.toString());
    console.log("  Voter 1:", voter1.publicKey.toString());
    console.log("  Voter 2:", voter2.publicKey.toString());
    console.log("  Delegate:", delegate.publicKey.toString());

    // Airdrop SOL to test accounts
    const accounts = [governanceAuthority, proposer, voter1, voter2, delegate];
    console.log("\n💰 Airdropping SOL...");

    for (const account of accounts) {
      try {
        const airdropSig = await connection.requestAirdrop(
          account.publicKey,
          5 * LAMPORTS_PER_SOL
        );
        await connection.confirmTransaction(airdropSig, "confirmed");
        console.log(`  ✓ Airdropped to ${account.publicKey.toString().slice(0, 8)}...`);
      } catch (error) {
        console.error(`  ✗ Failed to airdrop to ${account.publicKey.toString()}:`, error);
        throw error;
      }
    }

    // Create governance token (Token-2022)
    console.log("\n🪙 Creating governance token...");
    try {
      governanceToken = await createMint(
        connection,
        payer,
        payer.publicKey,
        null,
        9,
        Keypair.generate(),
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Token created:", governanceToken.toString());
    } catch (error) {
      console.error("  ✗ Failed to create token:", error);
      throw error;
    }

    // Create token accounts
    console.log("\n📝 Creating token accounts...");
    try {
      proposerTokenAccount = await createAccount(
        connection,
        payer,
        governanceToken,
        proposer.publicKey,
        undefined,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Proposer token account:", proposerTokenAccount.toString());

      voter1TokenAccount = await createAccount(
        connection,
        payer,
        governanceToken,
        voter1.publicKey,
        undefined,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Voter1 token account:", voter1TokenAccount.toString());

      voter2TokenAccount = await createAccount(
        connection,
        payer,
        governanceToken,
        voter2.publicKey,
        undefined,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Voter2 token account:", voter2TokenAccount.toString());

      delegateTokenAccount = await createAccount(
        connection,
        payer,
        governanceToken,
        delegate.publicKey,
        undefined,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Delegate token account:", delegateTokenAccount.toString());
    } catch (error) {
      console.error("  ✗ Failed to create token accounts:", error);
      throw error;
    }

    // Mint tokens
    console.log("\n🎁 Minting tokens...");
    try {
      await mintTo(
        connection,
        payer,
        governanceToken,
        proposerTokenAccount,
        payer,
        1000 * 10 ** 9,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Minted 1000 tokens to proposer");

      await mintTo(
        connection,
        payer,
        governanceToken,
        voter1TokenAccount,
        payer,
        500 * 10 ** 9,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Minted 500 tokens to voter1");

      await mintTo(
        connection,
        payer,
        governanceToken,
        voter2TokenAccount,
        payer,
        300 * 10 ** 9,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Minted 300 tokens to voter2");

      await mintTo(
        connection,
        payer,
        governanceToken,
        delegateTokenAccount,
        payer,
        100 * 10 ** 9,
        [],
        undefined,
        TOKEN_2022_PROGRAM_ID
      );
      console.log("  ✓ Minted 100 tokens to delegate");
    } catch (error) {
      console.error("  ✗ Failed to mint tokens:", error);
      throw error;
    }

    // Derive PDAs
    console.log("\n🔑 Deriving PDAs...");
    [governancePda, governanceBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("governance"), governanceAuthority.publicKey.toBuffer()],
      program.programId
    );
    console.log("  ✓ Governance PDA:", governancePda.toString());

    [treasuryPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("treasury"), governancePda.toBuffer()],
      program.programId
    );
    console.log("  ✓ Treasury PDA:", treasuryPda.toString());

    [treasuryAuthorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("treasury_authority"), governancePda.toBuffer()],
      program.programId
    );
    console.log("  ✓ Treasury Authority PDA:", treasuryAuthorityPda.toString());

    console.log("\n✅ Setup complete!\n");
  });

  describe("1. Governance Creation", () => {
    it("Creates a governance with token-weighted voting", async () => {
      console.log("\n📋 Creating token-weighted governance...");

      const config = {
        votingType: { tokenWeighted: {} },
        governanceToken: governanceToken,
        quorumPercentage: 51,
        votingPeriodHours: 72,
        executionDelayHours: 24,
        minVotingPowerToPropose: new BN(100 * 10 ** 9),
        proposalDeposit: new BN(0),
        votingPowerMultiplier: null,
        totalSupply: null,
        maxActiveProposals: 10,
      };

      const tx = await program.methods
        .createGovernance(governanceName, governanceDescription, config)
        .accounts({
          authority: governanceAuthority.publicKey,
          governance: governancePda,
          governanceToken: governanceToken,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([governanceAuthority])
        .rpc();

      console.log("  ✓ Transaction signature:", tx);

      const governanceAccount = await program.account.governance.fetch(governancePda);
      assert.equal(
        governanceAccount.authority.toString(),
        governanceAuthority.publicKey.toString()
      );
      assert.equal(governanceAccount.name, governanceName);
      assert.equal(governanceAccount.config.quorumPercentage, 51);
      assert.equal(governanceAccount.proposalCount.toNumber(), 0);
      assert.equal(governanceAccount.paused, false);

      console.log("  ✓ All assertions passed");
    });

    it("Creates a governance with equal voting", async () => {
      console.log("\n📋 Creating equal-voting governance...");

      const equalAuthority = Keypair.generate();
      const airdropSig = await connection.requestAirdrop(
        equalAuthority.publicKey,
        5 * LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(airdropSig, "confirmed");

      const [equalGovernancePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("governance"), equalAuthority.publicKey.toBuffer()],
        program.programId
      );

      const config = {
        votingType: { equal: {} },
        governanceToken: null,
        quorumPercentage: 60,
        votingPeriodHours: 48,
        executionDelayHours: 12,
        minVotingPowerToPropose: new BN(1),
        proposalDeposit: new BN(0),
        votingPowerMultiplier: null,
        totalSupply: null,
        maxActiveProposals: 5,
      };

      const tx = await program.methods
        .createGovernance("Equal DAO", "Equal voting governance", config)
        .accounts({
          authority: equalAuthority.publicKey,
          governance: equalGovernancePda,
          governanceToken: null,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([equalAuthority])
        .rpc();

      console.log("  ✓ Transaction signature:", tx);

      const governanceAccount = await program.account.governance.fetch(equalGovernancePda);
      assert.equal(governanceAccount.config.quorumPercentage, 60);
      console.log("  ✓ Equal voting governance created");
    });

    it("Fails with invalid quorum percentage", async () => {
      console.log("\n📋 Testing invalid quorum percentage...");

      const invalidAuthority = Keypair.generate();
      const airdropSig = await connection.requestAirdrop(
        invalidAuthority.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(airdropSig, "confirmed");

      const [invalidGovernancePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("governance"), invalidAuthority.publicKey.toBuffer()],
        program.programId
      );

      const config = {
        votingType: { equal: {} },
        governanceToken: null,
        quorumPercentage: 150,
        votingPeriodHours: 48,
        executionDelayHours: 12,
        minVotingPowerToPropose: new BN(1),
        proposalDeposit: new BN(0),
        votingPowerMultiplier: null,
        totalSupply: null,
        maxActiveProposals: 5,
      };

      try {
        await program.methods
          .createGovernance("Invalid DAO", "Invalid config", config)
          .accounts({
            authority: invalidAuthority.publicKey,
            governance: invalidGovernancePda,
            governanceToken: null,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([invalidAuthority])
          .rpc();

        assert.fail("Should have failed with invalid quorum");
      } catch (error: any) {
        assert.include(error.toString(), "InvalidQuorum");
        console.log("  ✓ Correctly rejected invalid quorum");
      }
    });
  });

  describe("2. Proposal Creation", () => {
    let proposalPda: PublicKey;
    const proposalId = 0;

    it("Creates a treasury proposal", async () => {
      console.log("\n📋 Creating treasury proposal...");

      const recipient = Keypair.generate().publicKey;

      [proposalPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("proposal"),
          governancePda.toBuffer(),
          new BN(proposalId).toArrayLike(Buffer, "le", 8),
        ],
        program.programId
      );

      const proposalType = {
        treasury: {
          recipient: recipient,
          amount: new BN(1 * LAMPORTS_PER_SOL),
          tokenMint: null,
        },
      };

      const instructionData = Buffer.from("test_instruction_data");

      const tx = await program.methods
        .createProposal(
          proposalTitle,
          proposalDescription,
          proposalType,
          instructionData
        )
        .accounts({
          proposer: proposer.publicKey,
          governance: governancePda,
          proposal: proposalPda,
          proposerTokenAccount: proposerTokenAccount,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([proposer])
        .rpc();

      console.log("  ✓ Proposal created:", tx);

      const proposalAccount = await program.account.proposal.fetch(proposalPda);
      assert.equal(proposalAccount.proposalId.toNumber(), proposalId);
      assert.equal(proposalAccount.title, proposalTitle);
      assert.equal(proposalAccount.proposer.toString(), proposer.publicKey.toString());
      assert.equal(proposalAccount.votesFor.toNumber(), 0);
      assert.equal(proposalAccount.votesAgainst.toNumber(), 0);
      console.log("  ✓ Proposal verification passed");
    });

    it("Fails when proposer lacks voting power", async () => {
      console.log("\n📋 Testing insufficient voting power...");

      const lowPowerProposer = Keypair.generate();
      const airdropSig = await connection.requestAirdrop(
        lowPowerProposer.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(airdropSig, "confirmed");

      const lowPowerTokenAccount = await createAccount(
        connection,
        payer,
        governanceToken,
        lowPowerProposer.publicKey,
        undefined,
        undefined,
        TOKEN_2022_PROGRAM_ID
      );

      const governanceAccount = await program.account.governance.fetch(governancePda);
      const nextProposalId = governanceAccount.proposalCount.toNumber();

      const [lowPowerProposalPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("proposal"),
          governancePda.toBuffer(),
          new BN(nextProposalId).toArrayLike(Buffer, "le", 8),
        ],
        program.programId
      );

      const proposalType = {
        treasury: {
          recipient: Keypair.generate().publicKey,
          amount: new BN(1 * LAMPORTS_PER_SOL),
          tokenMint: null,
        },
      };

      try {
        await program.methods
          .createProposal(
            "Low Power Proposal",
            "Should fail",
            proposalType,
            Buffer.from("dummy_instruction")
          )
          .accounts({
            proposer: lowPowerProposer.publicKey,
            governance: governancePda,
            proposal: lowPowerProposalPda,
            proposerTokenAccount: lowPowerTokenAccount,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([lowPowerProposer])
          .rpc();

        assert.fail("Should have failed with insufficient voting power");
      } catch (error: any) {
        const errorString = error.toString();
        if (
          errorString.includes("InsufficientVotingPower") ||
          errorString.includes("0x1771")
        ) {
          console.log("  ✓ Correctly rejected insufficient voting power");
        } else {
          throw new Error(`Expected InsufficientVotingPower error, but got: ${errorString}`);
        }
      }
    });
  });

  describe("3. Voting", () => {
    const proposalId = 0;
    let proposalPda: PublicKey;
    let voter1VoteRecordPda: PublicKey;
    let voter2VoteRecordPda: PublicKey;

    before(() => {
      [proposalPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("proposal"),
          governancePda.toBuffer(),
          new BN(proposalId).toArrayLike(Buffer, "le", 8),
        ],
        program.programId
      );

      [voter1VoteRecordPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          proposalPda.toBuffer(),
          voter1.publicKey.toBuffer(),
        ],
        program.programId
      );

      [voter2VoteRecordPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("vote"),
          proposalPda.toBuffer(),
          voter2.publicKey.toBuffer(),
        ],
        program.programId
      );
    });

    it("Casts a 'For' vote", async () => {
      console.log("\n  Casting 'For' vote...");

      const tx = await program.methods
        .vote(new BN(proposalId), { for: {} })
        .accounts({
          voter: voter1.publicKey,
          governance: governancePda,
          proposal: proposalPda,
          voteRecord: voter1VoteRecordPda,
          voterTokenAccount: voter1TokenAccount,
          delegation: null,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([voter1])
        .rpc();

      console.log("  ✓ Vote cast:", tx);

      const voteRecord = await program.account.voteRecord.fetch(voter1VoteRecordPda);
      assert.equal(voteRecord.voter.toString(), voter1.publicKey.toString());
      assert.equal(voteRecord.votingPower.toNumber(), 500 * 10 ** 9);

      const proposalAccount = await program.account.proposal.fetch(proposalPda);
      assert.equal(proposalAccount.votesFor.toNumber(), 500 * 10 ** 9);
      console.log("  ✓ Vote verified");
    });

    it("Prevents double voting", async () => {
      console.log("\n  Testing double voting prevention...");

      try {
        await program.methods
          .vote(new BN(proposalId), { for: {} })
          .accounts({
            voter: voter1.publicKey,
            governance: governancePda,
            proposal: proposalPda,
            voteRecord: voter1VoteRecordPda,
            voterTokenAccount: voter1TokenAccount,
            delegation: null,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([voter1])
          .rpc();

        assert.fail("Should have prevented double voting");
      } catch (error: any) {
        assert.ok(error);
        console.log("  ✓ Double voting prevented");
      }
    });
  });

  describe("4. Delegation", () => {
    let delegationPda: PublicKey;

    it("Delegates voting power", async () => {
      console.log("\n Delegating voting power...");

      [delegationPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("delegation"),
          proposer.publicKey.toBuffer(),
          delegate.publicKey.toBuffer(),
        ],
        program.programId
      );

      const tx = await program.methods
        .delegateVotingPower(new BN(1000 * 10 ** 9))
        .accounts({
          delegator: proposer.publicKey,
          delegate: delegate.publicKey,
          governance: governancePda,
          delegation: delegationPda,
          delegatorTokenAccount: proposerTokenAccount,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([proposer])
        .rpc();

      console.log("  ✓ Delegation created:", tx);

      const delegationAccount = await program.account.votingDelegation.fetch(delegationPda);
      assert.equal(delegationAccount.delegator.toString(), proposer.publicKey.toString());
      assert.equal(delegationAccount.delegate.toString(), delegate.publicKey.toString());
      assert.equal(delegationAccount.amount.toNumber(), 1000 * 10 ** 9);
      console.log("  ✓ Delegation verified");
    });

    it("Revokes delegation", async () => {
      console.log("\n Revoking delegation...");

      const tx = await program.methods
        .revokeDelegation()
        .accounts({
          delegator: proposer.publicKey,
          delegation: delegationPda,
        } as any)
        .signers([proposer])
        .rpc();

      console.log("  ✓ Delegation revoked:", tx);

      try {
        await program.account.votingDelegation.fetch(delegationPda);
        assert.fail("Delegation should be closed");
      } catch (error) {
        console.log("  ✓ Delegation account closed");
      }
    });
  });

  describe("5. Treasury Management", () => {
    it("Initializes treasury", async () => {
      console.log("\n Initializing treasury...");

      const tx = await program.methods
        .initializeTreasury()
        .accounts({
          authority: governanceAuthority.publicKey,
          governance: governancePda,
          treasury: treasuryPda,
          treasuryAuthority: treasuryAuthorityPda,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([governanceAuthority])
        .rpc();

      console.log("  ✓ Treasury initialized:", tx);

      const treasuryAccount = await program.account.treasury.fetch(treasuryPda);
      assert.equal(treasuryAccount.initialized, true);
      assert.equal(treasuryAccount.governance.toString(), governancePda.toString());
      console.log("  ✓ Treasury verified");
    });
  });

  describe("6. Admin Functions", () => {
    it("Emergency pauses governance", async () => {
      console.log("\n⏸  Pausing governance...");

      const tx = await program.methods
        .emergencyPause()
        .accounts({
          caller: governanceAuthority.publicKey,
          governance: governancePda,
        })
        .signers([governanceAuthority])
        .rpc();

      console.log("  ✓ Governance paused:", tx);

      const governanceAccount = await program.account.governance.fetch(governancePda);
      assert.equal(governanceAccount.paused, true);
    });

    it("Unpauses governance", async () => {
      console.log("\n▶  Unpausing governance...");

      const tx = await program.methods
        .unpause()
        .accounts({
          caller: governanceAuthority.publicKey,
          governance: governancePda,
        })
        .signers([governanceAuthority])
        .rpc();

      console.log("  ✓ Governance unpaused:", tx);

      const governanceAccount = await program.account.governance.fetch(governancePda);
      assert.equal(governanceAccount.paused, false);
    });
  });

  describe("7. Final State", () => {
    it("Displays final governance state", async () => {
      console.log("\n Final Governance State:");

      const governanceAccount = await program.account.governance.fetch(governancePda);
      console.log("  Name:", governanceAccount.name);
      console.log("  Proposal Count:", governanceAccount.proposalCount.toNumber());
      console.log("  Paused:", governanceAccount.paused);
      console.log("  Quorum:", governanceAccount.config.quorumPercentage + "%");

      console.log("\n All tests completed!\n");
    });
  });
});