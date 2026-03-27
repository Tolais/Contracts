# WebSocket Real-time Messaging Implementation

## Overview

This implementation adds real-time WebSocket support to the messaging system, enabling instant message delivery, typing indicators, and read receipts. The WebSocket server maintains persistent connections with users and facilitates bidirectional communication.

## Architecture

```
┌─────────────┐                          ┌─────────────┐
│   Fan       │                          │   Creator   │
│  Browser    │                          │   Browser   │
└──────┬──────┘                          └──────┬──────┘
       │                                        │
       │ WebSocket Connection                   │
       ├────────────────────────────────────────┤
       │                                        │
       ▼                                        ▼
┌─────────────────────────────────────────────────────────┐
│              Actix WebSocket Server                      │
│  ┌──────────────────────────────────────────────────┐   │
│  │ WsSession (Fan)        │  WsSession (Creator)    │   │
│  │ - Heartbeat monitoring │  - Message forwarding   │   │
│  │ - Message handling     │  - Typing indicators    │   │
│  └──────────────────────────────────────────────────┘   │
│                        │                                  │
│                        ▼                                  │
│            ┌──────────────────────┐                      │
│            │ MessageBroadcaster   │                      │
│            │ - Session registry   │                      │
│            │ - User lookup        │                      │
│            └──────────────────────┘                      │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
              ┌──────────────────┐
              │   PostgreSQL DB  │
              │ - Message storage│
              │ - Conversation   │
              └──────────────────┘
```

## Features

### 🔌 Real-time Communication

- **Instant Message Delivery**: Messages appear immediately in recipient's chat
- **Typing Indicators**: Show when the other party is typing
- **Read Receipts**: Real-time updates when messages are read
- **Online Status**: Track which users are currently connected

### 🔒 Security

- **Authentication**: JWT token required for WebSocket handshake
- **Session Isolation**: Each user can only access their own messages
- **Encrypted Payloads**: End-to-end encryption maintained over WebSocket
- **Rate Limiting**: Prevent spam/abuse of messaging system

### 💫 Performance

- **Heartbeat System**: Automatic ping/pong to detect disconnected clients
- **Connection Timeout**: 30-second timeout for unresponsive clients
- **Efficient Routing**: O(1) lookup for recipient sessions
- **Scalable Design**: Actor-based architecture for horizontal scaling

## WebSocket API

### Connection

Connect to the WebSocket endpoint:

```javascript
const userId = "user-uuid-here";
const token = "jwt-token-here"; // In production

const ws = new WebSocket(
  `ws://localhost:8081/ws?user_id=${userId}&token=${token}`
);
```

### Message Types

#### Send Message

Send an encrypted message to another user:

```javascript
ws.send(JSON.stringify({
  type: "SendMessage",
  recipient_id: "recipient-uuid",
  encrypted_content: "base64-ciphertext",
  nonce: "base64-nonce"
}));
```

**Server Response:**
```json
{
  "type": "Ack",
  "message_id": "msg-uuid",
  "status": "sent"
}
```

#### Mark as Read

Mark received messages as read:

```javascript
ws.send(JSON.stringify({
  type: "MarkRead",
  message_ids: ["msg-uuid-1", "msg-uuid-2"]
}));
```

**Server Response:**
```json
{
  "type": "Ack",
  "status": "read_receipt_sent"
}
```

#### Typing Indicator

Notify when user is typing:

```javascript
ws.send(JSON.stringify({
  type: "Typing",
  conversation_id: "conversation-uuid",
  is_typing: true
}));
```

#### Receive Message

When someone sends you a message:

```json
{
  "type": "NewMessage",
  "message_id": "msg-uuid",
  "sender_id": "sender-uuid",
  "encrypted_content": "base64-ciphertext",
  "nonce": "base64-nonce",
  "sent_at": "2026-03-26T15:30:00Z"
}
```

#### Error Handling

Error responses:

```json
{
  "type": "Error",
  "message": "Invalid message format: ..."
}
```

## Client Implementation Example

### React Hook Example

```javascript
import { useEffect, useState, useCallback } from 'react';

function useWebSocket(userId, token) {
  const [ws, setWs] = useState(null);
  const [messages, setMessages] = useState([]);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    const socket = new WebSocket(
      `ws://localhost:8081/ws?user_id=${userId}&token=${token}`
    );

    socket.onopen = () => {
      console.log('WebSocket connected');
      setIsConnected(true);
    };

    socket.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      
      switch (msg.type) {
        case 'NewMessage':
          setMessages(prev => [...prev, msg]);
          break;
        case 'Ack':
          console.log('Message acknowledged:', msg.message_id);
          break;
        case 'Error':
          console.error('WebSocket error:', msg.message);
          break;
      }
    };

    socket.onclose = () => {
      console.log('WebSocket disconnected');
      setIsConnected(false);
      
      // Auto-reconnect after 2 seconds
      setTimeout(() => {
        // Reconnect logic
      }, 2000);
    };

    setWs(socket);

    return () => {
      socket.close();
    };
  }, [userId, token]);

  const sendMessage = useCallback((recipientId, encryptedContent, nonce) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'SendMessage',
        recipient_id: recipientId,
        encrypted_content: encryptedContent,
        nonce: nonce
      }));
    }
  }, [ws]);

  const markAsRead = useCallback((messageIds) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'MarkRead',
        message_ids: messageIds
      }));
    }
  }, [ws]);

  const sendTypingIndicator = useCallback((conversationId, isTyping) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'Typing',
        conversation_id: conversationId,
        is_typing: isTyping
      }));
    }
  }, [ws]);

  return {
    ws,
    messages,
    isConnected,
    sendMessage,
    markAsRead,
    sendTypingIndicator
  };
}

// Usage in component
function ChatComponent({ currentUser, recipient }) {
  const { 
    isConnected, 
    sendMessage, 
    messages,
    markAsRead,
    sendTypingIndicator 
  } = useWebSocket(currentUser.id, currentUser.token);

  // Encrypt and send message
  const handleSendMessage = async (text) => {
    const { encrypted, nonce } = await encryptMessage(text, recipient.publicKey);
    sendMessage(recipient.id, encrypted, nonce);
  };

  return (
    <div>
      <div>Status: {isConnected ? 'Connected' : 'Disconnected'}</div>
      {/* Chat UI */}
    </div>
  );
}
```

## Server Configuration

### Heartbeat Settings

Configure in `websocket.rs`:

```rust
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);
```

### Scaling Considerations

For production deployment with multiple server instances:

1. **Redis Pub/Sub**: Share session state across servers
2. **Sticky Sessions**: Ensure WebSocket affinity to same server
3. **Message Queue**: Buffer messages for offline users

Example Redis integration:

```rust
pub struct RedisBackedBroadcaster {
    redis: redis::Client,
    local_sessions: HashMap<Uuid, Addr<WsSession>>,
}

impl RedisBackedBroadcaster {
    pub async fn publish(&self, user_id: Uuid, message: WsMessage) {
        // Publish to Redis channel
        self.redis.publish(user_id.to_string(), message).await?;
    }
    
    pub async fn subscribe(&self, user_id: Uuid) {
        // Subscribe to user's channel
        let mut subscriber = self.redis.subscribe(user_id.to_string()).await?;
        
        while let Some(msg) = subscriber.next_message().await? {
            // Forward to local session if exists
            if let Some(addr) = self.local_sessions.get(&user_id) {
                addr.do_send(/* forward message */);
            }
        }
    }
}
```

## Testing

### Manual Testing with wscat

Install wscat:
```bash
npm install -g wscat
```

Connect and test:
```bash
wscat -c "ws://localhost:8081/ws?user_id=test-user-uuid"

# Send a message
> {"type":"SendMessage","recipient_id":"other-uuid","encrypted_content":"test","nonce":"abc"}

# Receive acknowledgment
< {"type":"Ack","message_id":"uuid","status":"sent"}
```

### Automated Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_rt::test]
    async fn test_websocket_connection() {
        let app = test::init_service(
            App::new()
                .route("/ws", web::get().to(websocket::ws_route))
        ).await;

        // Test WebSocket handshake
        let req = test::TestRequest::get()
            .uri("/ws?user_id=test-uuid")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
```

## Monitoring & Metrics

Track these metrics in production:

- **Active Connections**: Current WebSocket sessions
- **Message Throughput**: Messages sent/received per second
- **Latency**: Time from send to receive
- **Error Rate**: Failed message deliveries
- **Reconnection Rate**: How often clients reconnect

Example Prometheus metrics:

```rust
use prometheus::{IntGauge, Counter, Histogram};

static ACTIVE_CONNECTIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("ws_active_connections", "Number of active WS connections").unwrap()
});

static MESSAGES_SENT: Lazy<Counter> = Lazy::new(|| {
    register_counter!("ws_messages_sent_total", "Total messages sent").unwrap()
});

static MESSAGE_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!("ws_message_latency_seconds", "Message delivery latency").unwrap()
});
```

## Troubleshooting

### Connection Issues

**Problem**: WebSocket fails to connect
- Check CORS settings allow WebSocket upgrade
- Verify user_id parameter is valid UUID format
- Ensure JWT token is valid (if enabled)

**Problem**: Frequent disconnections
- Increase `CLIENT_TIMEOUT` value
- Check network stability
- Monitor server resource usage (memory/CPU)

### Message Delivery Issues

**Problem**: Messages not delivered
- Verify recipient is online (check session registry)
- Check message serialization (JSON format)
- Review server logs for errors

**Problem**: High latency
- Profile message routing path
- Check database query performance
- Consider adding caching layer

## Future Enhancements

- [ ] File transfer over WebSocket (encrypted)
- [ ] Voice/video call signaling
- [ ] Group chat support
- [ ] Message reactions (emoji)
- [ ] Presence subscriptions
- [ ] Offline message queue
- [ ] End-to-end encrypted group calls

---

**Version**: 0.1.0  
**Last Updated**: 2026-03-26
