import { CosmWasmClient, SigningCosmWasmClient, Secp256k1HdWallet } from "cosmwasm";
import { assert, expect } from "chai";
import { InteractionType, PostType } from "./types";

describe("PostMinter Contract Tests", () => {
  const contractAddress = process.env.POST_MINTER_ADDRESS || "";
  const rpcEndpoint = process.env.RPC_ENDPOINT || "http://localhost:26657";
  
  let client: CosmWasmClient;
  let signingClient: SigningCosmWasmClient;
  let wallet: Secp256k1HdWallet;
  let unauthorizedWallet: Secp256k1HdWallet;
  let userAddress: string;
  let unauthorizedAddress: string;
  
  // Test variables
  const validTribeId = 1;
  const invalidTribeId = 9999;
  let createdPostId: number;
  let encryptedPostId: number;
  
  before(async () => {
    client = await CosmWasmClient.connect(rpcEndpoint);
    
    // Set up wallet for testing
    const mnemonic = process.env.TEST_MNEMONIC || "your test mnemonic here";
    wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic);
    const [firstAccount] = await wallet.getAccounts();
    userAddress = firstAccount.address;
    
    // Set up an unauthorized wallet
    const unauthorizedMnemonic = "test test test test test test test test test test test junk";
    unauthorizedWallet = await Secp256k1HdWallet.fromMnemonic(unauthorizedMnemonic);
    const [unauthorizedAccount] = await unauthorizedWallet.getAccounts();
    unauthorizedAddress = unauthorizedAccount.address;
    
    signingClient = await SigningCosmWasmClient.connectWithSigner(
      rpcEndpoint,
      wallet
    );
  });
  
  describe("Instantiation", () => {
    it("should successfully connect to the contract", async () => {
      const contractInfo = await client.getContract(contractAddress);
      expect(contractInfo).to.exist;
      expect(contractInfo.address).to.equal(contractAddress);
    });
  });
  
  describe("Post Creation - Success Cases", () => {
    it("should create a text post", async () => {
      const metadata = JSON.stringify({
        title: "Test Post",
        content: "This is a test post",
        type: "TEXT"
      });
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Extract post_id from logs
      const postIdLog = result.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "post_id");
      
      expect(postIdLog).to.exist;
      createdPostId = parseInt(postIdLog?.value || "0");
      expect(createdPostId).to.be.a("number");
      expect(createdPostId).to.be.greaterThan(0);
    });
    
    it("should create a rich media post", async () => {
      const metadata = JSON.stringify({
        title: "Rich Media Post",
        content: "This is a rich media post",
        type: "RICH_MEDIA",
        media_url: "https://example.com/image.jpg"
      });
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should create an encrypted post", async () => {
      const metadata = JSON.stringify({
        title: "Encrypted Post",
        content: "This is an encrypted post",
        type: "ENCRYPTED"
      });
      
      const keyHash = "0x1234567890abcdef";
      const accessSigner = userAddress; // Using own address as signer for test
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_encrypted_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            encryption_key_hash: keyHash,
            access_signer: accessSigner
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Extract post_id from logs
      const postIdLog = result.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "post_id");
      
      encryptedPostId = parseInt(postIdLog?.value || "0");
    });
    
    it("should create a reply to an existing post", async () => {
      const metadata = JSON.stringify({
        title: "Reply Post",
        content: "This is a reply to the original post",
        type: "TEXT"
      });
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_reply: {
            parent_post_id: createdPostId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
  });
  
  describe("Post Creation - Failure Cases", () => {
    it("should fail when creating a post with invalid tribe ID", async () => {
      const metadata = JSON.stringify({
        title: "Invalid Tribe Post",
        content: "This post should fail",
        type: "TEXT"
      });
      
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            create_post: {
              tribe_id: invalidTribeId,
              metadata: metadata,
              is_gated: false,
              collectible_contract: null,
              collectible_id: 0
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        // Check error message contains expected text
        expect((error as Error).toString()).to.include("Not tribe member");
      }
    });
    
    it("should fail when creating a post with empty metadata", async () => {
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            create_post: {
              tribe_id: validTribeId,
              metadata: "",
              is_gated: false,
              collectible_contract: null,
              collectible_id: 0
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Empty metadata");
      }
    });
    
    it("should fail when replying to a non-existent post", async () => {
      const metadata = JSON.stringify({
        title: "Invalid Reply",
        content: "This reply should fail",
        type: "TEXT"
      });
      
      const nonExistentPostId = 999999;
      
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            create_reply: {
              parent_post_id: nonExistentPostId,
              metadata: metadata,
              is_gated: false,
              collectible_contract: null,
              collectible_id: 0
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Invalid parent post");
      }
    });
    
    it("should fail when creating an encrypted post with invalid encryption key", async () => {
      const metadata = JSON.stringify({
        title: "Invalid Encrypted Post",
        content: "This post should fail",
        type: "ENCRYPTED"
      });
      
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            create_encrypted_post: {
              tribe_id: validTribeId,
              metadata: metadata,
              encryption_key_hash: "",
              access_signer: userAddress
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Invalid encryption key");
      }
    });
  });
  
  describe("Post Interactions - Success Cases", () => {
    it("should like a post", async () => {
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          interact_with_post: {
            post_id: createdPostId,
            interaction_type: InteractionType.LIKE
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should report a post", async () => {
      // Create a new post to report
      const metadata = JSON.stringify({
        title: "Post to Report",
        content: "This post will be reported",
        type: "TEXT"
      });
      
      const createResult = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      const postIdLog = createResult.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "post_id");
      
      const postToReport = parseInt(postIdLog?.value || "0");
      
      // Report the post
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          report_post: {
            post_id: postToReport,
            reason: "Test report reason"
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should authorize a viewer for an encrypted post", async () => {
      // Using a second account as viewer
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          authorize_viewer: {
            post_id: encryptedPostId,
            viewer: unauthorizedAddress
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
  });
  
  describe("Post Interactions - Failure Cases", () => {
    it("should fail when interacting with own post", async () => {
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            interact_with_post: {
              post_id: createdPostId,
              interaction_type: InteractionType.LIKE
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Cannot interact with own post");
      }
    });
    
    it("should fail when interacting with non-existent post", async () => {
      const nonExistentPostId = 999999;
      
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            interact_with_post: {
              post_id: nonExistentPostId,
              interaction_type: InteractionType.LIKE
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
      }
    });
    
    it("should fail when unauthorized user tries to delete a post", async () => {
      // Connect with unauthorized wallet
      const unauthorizedClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        unauthorizedWallet
      );
      
      try {
        await unauthorizedClient.execute(
          unauthorizedAddress,
          contractAddress,
          {
            delete_post: {
              post_id: createdPostId
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Not post creator");
      }
    });
    
    it("should fail when reporting a post twice", async () => {
      // Create a new post to report
      const metadata = JSON.stringify({
        title: "Post to Report Twice",
        content: "This post will be reported twice",
        type: "TEXT"
      });
      
      const createResult = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      const postIdLog = createResult.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "post_id");
      
      const postToReport = parseInt(postIdLog?.value || "0");
      
      // Report the post first time
      await signingClient.execute(
        userAddress,
        contractAddress,
        {
          report_post: {
            post_id: postToReport,
            reason: "Test report reason"
          }
        },
        "auto"
      );
      
      // Try to report the same post again
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            report_post: {
              post_id: postToReport,
              reason: "Test report reason again"
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Already reported");
      }
    });
  });
  
  describe("Post Management - Success Cases", () => {
    it("should delete own post", async () => {
      // Create a new post to delete
      const metadata = JSON.stringify({
        title: "Post to Delete",
        content: "This post will be deleted",
        type: "TEXT"
      });
      
      const createResult = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      const postIdLog = createResult.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "post_id");
      
      const postToDelete = parseInt(postIdLog?.value || "0");
      
      // Delete the post
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          delete_post: {
            post_id: postToDelete
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should update post metadata", async () => {
      // Create a new post to update
      const metadata = JSON.stringify({
        title: "Post to Update",
        content: "This post will be updated",
        type: "TEXT"
      });
      
      const createResult = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      const postIdLog = createResult.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "post_id");
      
      const postToUpdate = parseInt(postIdLog?.value || "0");
      
      // Update the post
      const updatedMetadata = JSON.stringify({
        title: "Updated Post",
        content: "This post has been updated",
        type: "TEXT"
      });
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          update_post: {
            post_id: postToUpdate,
            metadata: updatedMetadata
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
  });
  
  describe("Post Management - Failure Cases", () => {
    let deletedPostId: number;
    
    before(async () => {
      // Create and delete a post for testing
      const metadata = JSON.stringify({
        title: "Post to Delete for Tests",
        content: "This post will be deleted for testing failures",
        type: "TEXT"
      });
      
      const createResult = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_post: {
            tribe_id: validTribeId,
            metadata: metadata,
            is_gated: false,
            collectible_contract: null,
            collectible_id: 0
          }
        },
        "auto"
      );
      
      const postIdLog = createResult.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "post_id");
      
      deletedPostId = parseInt(postIdLog?.value || "0");
      
      // Delete the post
      await signingClient.execute(
        userAddress,
        contractAddress,
        {
          delete_post: {
            post_id: deletedPostId
          }
        },
        "auto"
      );
    });
    
    it("should fail when deleting a post that's already deleted", async () => {
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            delete_post: {
              post_id: deletedPostId
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Post deleted");
      }
    });
    
    it("should fail when updating a deleted post", async () => {
      const updatedMetadata = JSON.stringify({
        title: "Updated Deleted Post",
        content: "This update should fail",
        type: "TEXT"
      });
      
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            update_post: {
              post_id: deletedPostId,
              metadata: updatedMetadata
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Post deleted");
      }
    });
    
    it("should fail when replying to a deleted post", async () => {
      const metadata = JSON.stringify({
        title: "Reply to Deleted Post",
        content: "This reply should fail",
        type: "TEXT"
      });
      
      try {
        await signingClient.execute(
          userAddress,
          contractAddress,
          {
            create_reply: {
              parent_post_id: deletedPostId,
              metadata: metadata,
              is_gated: false,
              collectible_contract: null,
              collectible_id: 0
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Post deleted");
      }
    });
  });
  
  describe("Query Functions - Success Cases", () => {
    it("should get a post", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_post: {
          post_id: createdPostId
        }
      });
      
      expect(result).to.exist;
      expect(result.id).to.equal(createdPostId);
      expect(result.metadata).to.be.a("string");
    });
    
    it("should get interaction count", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_interaction_count: {
          post_id: createdPostId,
          interaction_type: InteractionType.LIKE
        }
      });
      
      expect(result).to.exist;
      expect(result.count).to.be.a("number");
    });
    
    it("should check if user can view post", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        can_view_post: {
          post_id: createdPostId,
          viewer: userAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.result).to.be.true;
    });
    
    it("should get post replies", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_post_replies: {
          post_id: createdPostId
        }
      });
      
      expect(result).to.exist;
      expect(result.replies).to.be.an("array");
    });
    
    it("should get posts by tribe", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_posts_by_tribe: {
          tribe_id: validTribeId,
          offset: 0,
          limit: 10
        }
      });
      
      expect(result).to.exist;
      expect(result.posts).to.be.an("array");
      expect(result.total).to.be.a("number");
    });
    
    it("should get posts by user", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_posts_by_user: {
          user: userAddress,
          offset: 0,
          limit: 10
        }
      });
      
      expect(result).to.exist;
      expect(result.posts).to.be.an("array");
      expect(result.total).to.be.a("number");
    });
  });
  
  describe("Query Functions - Failure Cases", () => {
    it("should return empty result when getting non-existent post", async () => {
      try {
        await client.queryContractSmart(contractAddress, {
          get_post: {
            post_id: 999999
          }
        });
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
      }
    });
    
    it("should return false when unauthorized user tries to view encrypted post", async () => {
      // Create a random address that isn't authorized
      const randomAddress = "cosmos1randomaddress123456789abcdef";
      
      const result = await client.queryContractSmart(contractAddress, {
        can_view_post: {
          post_id: encryptedPostId,
          viewer: randomAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.result).to.be.false;
    });
    
    it("should fail to get decryption key for unauthorized user", async () => {
      // Create a random address that isn't authorized
      const randomAddress = "cosmos1randomaddress123456789abcdef";
      
      const result = await client.queryContractSmart(contractAddress, {
        get_post_decryption_key: {
          post_id: encryptedPostId,
          viewer: randomAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.value).to.equal(""); // Empty key for unauthorized user
    });
  });
  
  describe("Post Type Functions", () => {
    it("should validate post metadata", async () => {
      const validMetadata = JSON.stringify({
        title: "Valid Post",
        content: "This is valid content",
        type: "TEXT"
      });
      
      const result = await client.queryContractSmart(contractAddress, {
        validate_metadata: {
          metadata: validMetadata,
          post_type: PostType.TEXT
        }
      });
      
      expect(result).to.exist;
      expect(result.result).to.be.true;
    });
    
    it("should reject invalid post metadata", async () => {
      const invalidMetadata = JSON.stringify({
        title: "",  // Empty title
        content: "This has an empty title",
        type: "TEXT"
      });
      
      const result = await client.queryContractSmart(contractAddress, {
        validate_metadata: {
          metadata: invalidMetadata,
          post_type: PostType.TEXT
        }
      });
      
      expect(result).to.exist;
      expect(result.result).to.be.false;
    });
    
    it("should get post type cooldown", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_post_type_cooldown: {
          post_type: PostType.TEXT
        }
      });
      
      expect(result).to.exist;
      expect(result.cooldown).to.be.a("number");
    });
    
    it("should get batch posting limits", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_batch_posting_limits: {}
      });
      
      expect(result).to.exist;
      expect(result.max_batch_size).to.be.a("number");
      expect(result.batch_cooldown).to.be.a("number");
    });
  });
  
  describe("Admin Functions", () => {
    it("should set post type cooldown if authorized", async () => {
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          set_post_type_cooldown: {
            post_type: PostType.TEXT,
            cooldown: 120
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should fail to set post type cooldown if unauthorized", async () => {
      // Connect with unauthorized wallet
      const unauthorizedClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        unauthorizedWallet
      );
      
      try {
        await unauthorizedClient.execute(
          unauthorizedAddress,
          contractAddress,
          {
            set_post_type_cooldown: {
              post_type: PostType.TEXT,
              cooldown: 120
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Not rate limit manager");
      }
    });
  });
}); 