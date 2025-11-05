import React, { useState, useRef, useEffect } from 'react';
import { Model, Message } from '../../shared/types';
import { formatBytes } from '../../shared/utils';

interface ChatProps {
  models: Model[];
}

const Chat: React.FC<ChatProps> = ({ models }) => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const loadedModels = models.filter((m) => m.loaded);
  const selectedModel = loadedModels[0];

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || loading || !selectedModel) return;

    const userMessage: Message = {
      id: `msg-${Date.now()}`,
      role: 'user',
      content: input,
      createdAt: Date.now(),
    };

    setMessages([...messages, userMessage]);
    setInput('');
    setLoading(true);

    try {
      const result = await window.electronAPI.sendMessage({
        conversationId: 'default',
        message: userMessage,
      });

      if (result.success) {
        setMessages((prev) => [...prev, result.message]);
      } else {
        // Show error message
        const errorMessage: Message = {
          id: `msg-${Date.now()}`,
          role: 'assistant',
          content: `Error: ${result.error}`,
          createdAt: Date.now(),
        };
        setMessages((prev) => [...prev, errorMessage]);
      }
    } catch (error) {
      console.error('Error sending message:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border p-4">
        <h2 className="text-lg font-semibold">Chat</h2>
        {selectedModel ? (
          <p className="text-sm text-muted-foreground mt-1">
            Using: {selectedModel.name} ({selectedModel.quantization || 'N/A'})
          </p>
        ) : (
          <p className="text-sm text-destructive mt-1">
            No model loaded. Go to Models page to load a model.
          </p>
        )}
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            <div className="text-center">
              <p className="text-xl mb-2">Start a conversation</p>
              <p className="text-sm">Type a message below to get started</p>
            </div>
          </div>
        ) : (
          messages.map((message) => (
            <div
              key={message.id}
              className={`flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}
            >
              <div
                className={`max-w-[80%] rounded-lg p-3 ${
                  message.role === 'user'
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-foreground'
                }`}
              >
                <div className="text-xs font-semibold mb-1 opacity-70">
                  {message.role === 'user' ? 'You' : 'Assistant'}
                </div>
                <div className="whitespace-pre-wrap">{message.content}</div>
              </div>
            </div>
          ))
        )}
        {loading && (
          <div className="flex justify-start">
            <div className="bg-muted rounded-lg p-3">
              <div className="flex space-x-2">
                <div className="w-2 h-2 bg-foreground rounded-full animate-bounce"></div>
                <div
                  className="w-2 h-2 bg-foreground rounded-full animate-bounce"
                  style={{ animationDelay: '0.1s' }}
                ></div>
                <div
                  className="w-2 h-2 bg-foreground rounded-full animate-bounce"
                  style={{ animationDelay: '0.2s' }}
                ></div>
              </div>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="border-t border-border p-4">
        <div className="flex space-x-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={
              selectedModel ? 'Type a message... (Enter to send, Shift+Enter for new line)' : 'Load a model first...'
            }
            disabled={!selectedModel || loading}
            className="flex-1 bg-background border border-border rounded-lg p-3 resize-none focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
            rows={3}
          />
          <button
            onClick={handleSend}
            disabled={!selectedModel || loading || !input.trim()}
            className="px-6 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
};

export default Chat;
