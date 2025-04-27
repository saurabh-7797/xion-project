// Common types used across all test files

export enum PostType {
  TEXT = "TEXT",
  RICH_MEDIA = "RICH_MEDIA",
  EVENT = "EVENT",
  POLL = "POLL",
  PROJECT_UPDATE = "PROJECT_UPDATE",
  COMMUNITY_UPDATE = "COMMUNITY_UPDATE",
  ENCRYPTED = "ENCRYPTED"
}

export enum InteractionType {
  LIKE = "LIKE",
  DISLIKE = "DISLIKE",
  SHARE = "SHARE",
  REPORT = "REPORT",
  REPLY = "REPLY"
}

export enum JoinType {
  PUBLIC = "PUBLIC",
  PRIVATE = "PRIVATE",
  INVITE_CODE = "INVITE_CODE",
  NFT_GATED = "NFT_GATED",
  MULTI_NFT = "MULTI_NFT",
  ANY_NFT = "ANY_NFT"
}

export enum NFTType {
  ERC721 = "ERC721",
  ERC1155 = "ERC1155"
}

export enum MemberStatus {
  NONE = "NONE",
  PENDING = "PENDING",
  ACTIVE = "ACTIVE",
  BANNED = "BANNED"
}

export interface NFTRequirement {
  nft_contract: string | null;
  nft_type: NFTType;
  is_mandatory: boolean;
  min_amount: number;
  token_ids: number[];
}

export interface BatchPostData {
  metadata: string;
  is_gated: boolean;
  collectible_contract: string | null;
  collectible_id: number;
  post_type: PostType;
}

export interface PostMetadata {
  title: string;
  content: string;
  type: string;
  [key: string]: any;
}

export interface NFTMetadata {
  name: string;
  description: string;
  image: string;
  attributes: {
    trait_type: string;
    value: string;
  }[];
  [key: string]: any;
}

// Response types

export interface BoolResponse {
  result: boolean;
}

export interface CountResponse {
  count: number;
}

export interface StringResponse {
  value: string;
}

export interface PostResponse {
  id: number;
  creator: string;
  tribe_id: number;
  metadata: string;
  is_gated: boolean;
  collectible_contract?: string;
  collectible_id: number;
  is_encrypted: boolean;
  access_signer?: string;
}

export interface PostsResponse {
  posts: number[];
  total: number;
}

export interface TokensResponse {
  tokens: string[];
}

export interface NFTInfoResponse {
  token_uri: string;
  extension: any;
}

export interface OwnerResponse {
  owner: string;
}

export interface MemberStatusResponse {
  status: MemberStatus;
}

export interface WhitelistResponse {
  whitelist: string[];
}

export interface TribeConfigResponse {
  config: {
    join_type: JoinType;
    entry_fee: string;
    nft_requirements: NFTRequirement[];
    can_merge: boolean;
  };
}

export interface InviteCodeStatusResponse {
  valid: boolean;
  remaining_uses: number;
} 