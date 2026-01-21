1. Review the claude.md file and resources provided.
2. Suggest a new type system, showing examples of the new types implemented for a signed action and user signed action.
Think hard about how an advanced rust developer would implement this


I want this type system example to include the following types 
* L1Action trait - supports serializing l1 action payload
* UserSignedAction trait - supports custom eip712 signing
* Action trait - supports building signing hash(from self, SigningChain, and other metadata held by client(nonce/timestamp, vault, expires_after)), and serializing action to be submitted to exchange api.
* Action trait should be implemented as a generic for types that implement L1Action or UserSignedAction traits.
* UsdSend struct implementing UserSignedAction trait
* EnableBigBlocks struct implementing UserSignedAction trait
* Exchange Client that sends requests implementing the Action trait

write to output.md
