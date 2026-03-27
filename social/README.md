# Social Backend - Exclusive Comment System & Messaging

## Overview

This backend provides an exclusive community platform where only fans with active subscriptions can participate in comment sections, and premium tier subscribers get access to direct messaging with creators. All messages are end-to-end encrypted.

## Features

### 💬 Exclusive Comment System

**Access Control:**
- Only fans with **active subscriptions** can view/comment
- Creates an "Exclusive Club" atmosphere free from trolls
- Threaded comments for organized discussions
- Like system for community curation

**Comment Features:**
- ✅ Create comments (top-level or replies)
- ✅ Edit own comments (marked as edited)
- ✅ Delete comments (soft delete)
- ✅ Like/unlike comments
- ✅ Nested reply threads
- ✅ Pagination support

### 🔒 E2E Encrypted Messaging

**Security:**
- ChaCha20-Poly1305 encryption
- Client-side key management
- Server stores only encrypted blobs
- Perfect forward secrecy support

**Access Control:**
- **Gold Tier (Level 3+)** required to DM creators
- Prevents spam from low-tier subscribers
- Adds value to premium subscriptions

**Messaging Features:**
- ✅ Send encrypted messages
- ✅ View conversation list
- ✅ Message history per user
- ✅ Read receipts
- ✅ Soft delete messages
- ✅ Unread message counting

## API Endpoints

### Comment System

#### POST `/api/v1/comments`

Create a new comment (requires active subscription).

**Headers:**
```
X-User-ID: <fan_uuid>
```

**Request:**
```json
{
  "creator_id": "uuid-of-creator",
  "content": "Great content! Love this.",
  "parent_comment_id": null // Optional for replies
}
```

**Response (403 if no subscription):**
```json
{
  "error": "Active subscription required to comment"
}
```

#### GET `/api/v1/comments/{creator_id}`

Get all comments for a creator (threaded).

**Query Parameters:**
- `page` (default: 1)
- `per_page` (default: 20)

**Response:**
```json
{
  "comments": [
    {
      "id": "uuid",
      "creator_id": "uuid",
      "fan_id": "uuid",
      "parent_comment_id": null,
      "content": "Great content!",
      "is_edited": false,
      "is_deleted": false,
      "like_count": 15,
      "created_at": "2026-03-26T14:30:00Z",
      "replies": [
        {
          "id": "reply-uuid",
          "parent_comment_id": "uuid",
          "content": "Thanks!",
          "like_count": 3,
          ...
        }
      ]
    }
  ],
  "total_count": 150,
  "page": 1,
  "per_page": 20,
  "has_more": true
}
```

#### PUT `/api/v1/comments/{comment_id}`

Update your own comment.

**Request:**
```json
{
  "content": "Updated comment text"
}
```

#### DELETE `/api/v1/comments/{comment_id}`

Soft delete a comment.

#### POST `/api/v1/comments/{comment_id}/like`

Like a comment.

**Response:**
```json
{
  "comment_id": "uuid",
  "like_count": 16
}
```

### Messaging System

#### POST `/api/v1/messages`

Send an encrypted message.

**Headers:**
```
X-User-ID: <sender_uuid>
```

**Request:**
```json
{
  "recipient_id": "creator-uuid",
  "encrypted_content": "base64-encoded-ciphertext",
  "nonce": "base64-encoded-nonce"
}
```

**Response (403 if below Gold tier):**
```json
{
  "error": "Gold tier subscription required to message this creator"
}
```

#### GET `/api/v1/messages/conversations`

Get user's conversation list.

**Response:**
```json
[
  {
    "id": "conversation-uuid",
    "participant_id": "other-user-uuid",
    "participant_name": "CreatorName",
    "last_message_at": "2026-03-26T15:00:00Z",
    "last_message_preview": "[Encrypted Message]",
    "unread_count": 3
  }
]
```

#### GET `/api/v1/messages/{recipient_id}`

Get message history with a specific user.

**Response:**
```json
[
  {
    "id": "message-uuid",
    "sender_id": "sender-uuid",
    "recipient_id": "recipient-uuid",
    "encrypted_content": "base64-ciphertext",
    "nonce": "base64-nonce",
    "is_read": true,
    "read_at": "2026-03-26T15:00:00Z",
    "sent_at": "2026-03-26T14:30:00Z"
  }
]
```

#### PUT `/api/v1/messages/{message_id}/read`

Mark a message as read.

#### DELETE `/api/v1/messages/{message_id}`

Soft delete a message.

## Database Schema

### Key Tables

- **users**: Base user accounts with public keys
- **creators**: Creator profiles
- **fans**: Fan profiles  
- **subscription_tiers**: Tier levels (Bronze/Silver/Gold)
- **subscriptions**: Active fan subscriptions
- **comments**: Threaded comments (gated by subscription)
- **comment_likes**: Like tracking
- **messages**: E2E encrypted messages
- **conversations**: Conversation metadata

### Subscription Tiers

| Level | Name | Permissions |
|-------|------|-------------|
| 1 | Bronze | View comments |
| 2 | Silver | Comment access |
| 3+ | Gold | DM creator access |

## Security Model

### Comment Gating

Comments are protected by database constraint:

```sql
CONSTRAINT check_active_subscription CHECK (
    EXISTS (
        SELECT 1 FROM subscriptions s 
        WHERE s.fan_id = fans.user_id 
          AND s.creator_id = comments.creator_id 
          AND s.status = 'active'
    )
)
```

### E2E Encryption Flow

```
┌──────────┐                          ┌──────────┐
│  Sender  │                          │ Recipient│
└────┬─────┘                          └────┬─────┘
     │                                     │
     │ 1. Generate ephemeral keypair       │
     │ 2. Derive shared secret             │
     │ 3. Encrypt with ChaCha20            │
     │                                     │
     ├─────────► Server ◄──────────────────┤
     │           (stores encrypted)         │
     │                                     │
     │ 4. Send ciphertext                  │
     │                                     │
     │                              5. Decrypt with shared secret
```

### Client-Side Encryption Example

```javascript
// Using TweetNaCl.js
const nacl = require('tweetnacl');

// Generate keypair (store securely)
const keypair = nacl.box.keyPair();

// Derive shared secret
const sharedSecret = nacl.box.before(
  recipientPublicKey,
  keypair.secretKey
);

// Encrypt message
const nonce = nacl.randomBytes(nacl.box.nonceLength);
const encrypted = nacl.box.after(
  utf8ToUint8Array(message),
  nonce,
  sharedSecret
);

// Send to server
await fetch('/api/v1/messages', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'X-User-ID': userId
  },
  body: JSON.stringify({
    recipient_id: recipientId,
    encrypted_content: btoa(String.fromCharCode(...encrypted)),
    nonce: btoa(String.fromCharCode(...nonce))
  })
});
```

## Setup & Installation

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- OpenSSL

### Database Setup

```bash
# Create database
createdb stellar_social

# Apply schema
psql stellar_social < db/schema.sql
```

### Configuration

Create `.env` file:

```env
DATABASE_URL=postgres://user:password@localhost/stellar_social
RUST_LOG=info,actix_web=debug
```

### Running the Server

```bash
# Development
cargo run

# Release
cargo run --release

# Tests
cargo test
```

Server runs on `http://0.0.0.0:8081`

## Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# With coverage
cargo tarpaulin --out Html
```

## Performance Considerations

### Indexing Strategy

- Comments indexed by creator and creation date
- Messages indexed by sender/recipient
- Subscriptions indexed for fast eligibility checks

### Caching Recommendations

- Cache subscription status (Redis, 5min TTL)
- Cache comment counts per creator
- Preload conversation metadata

## Future Enhancements

- [ ] WebSocket support for real-time chat
- [ ] File attachments in messages (encrypted)
- [ ] Comment moderation tools for creators
- [ ] Block/mute users functionality
- [ ] Rich text editor support
- [ ] Emoji reactions beyond likes
- [ ] Notification system for new messages/comments

## Troubleshooting

**Cannot comment (403 error):**
- Verify fan has active subscription to creator
- Check subscription status in database

**Message sending fails:**
- Ensure sender has Gold tier (level 3+) for creator DMs
- Verify encryption is done client-side

**Slow comment loading:**
- Add database indexes on `comments(creator_id, created_at)`
- Implement pagination for large threads

---

**Version**: 0.1.0  
**Last Updated**: 2026-03-26
