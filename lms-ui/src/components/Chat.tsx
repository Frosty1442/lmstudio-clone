import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { NoChatsEmpty } from "./EmptyState";

interface Message {
  role: string;
  content: string;
}

interface ChatProps {
  models: any[];
  onError?: (title: string, message: string) => void;
}

export default function Chat({ models, onError }: ChatProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const loadedModels = models.filter((m) => m.loaded);
  const selectedModel = loadedModels[0];

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || loading || !selectedModel) return;

    const userMessage: Message = {
      role: "user",
      content: input,
    };

    const newMessages = [...messages, userMessage];
    setMessages(newMessages);
    setInput("");
    setLoading(true);

    try {
      const response: any = await invoke("send_message", {
        model: selectedModel.id,
        messages: newMessages.map((m) => ({ role: m.role, content: m.content })),
      });

      const assistantMessage: Message = {
        role: "assistant",
        content: response.choices[0].message.content,
      };

      setMessages([...newMessages, assistantMessage]);
    } catch (error) {
      console.error("Error sending message:", error);
      onError?.("Message Failed", String(error));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-gray-800 p-4">
        <h2 className="text-lg font-semibold">Chat</h2>
        {selectedModel ? (
          <p className="text-sm text-gray-400 mt-1">
            Using: {selectedModel.name} ({selectedModel.quantization || "N/A"})
          </p>
        ) : (
          <p className="text-sm text-red-500 mt-1">
            No model loaded. Go to Models page to load one.
          </p>
        )}
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <NoChatsEmpty />
        ) : (
          messages.map((msg, idx) => (
            <div
              key={idx}
              className={`flex ${
                msg.role === "user" ? "justify-end" : "justify-start"
              }`}
            >
              <div
                className={`max-w-[80%] rounded-lg p-3 ${
                  msg.role === "user"
                    ? "bg-blue-600 text-white"
                    : "bg-gray-800 text-gray-100"
                }`}
              >
                <div className="text-xs font-semibold mb-1 opacity-70">
                  {msg.role === "user" ? "You" : "Assistant"}
                </div>
                <div className="whitespace-pre-wrap">{msg.content}</div>
              </div>
            </div>
          ))
        )}
        {loading && (
          <div className="flex justify-start">
            <div className="bg-gray-800 rounded-lg p-3">
              <div className="flex space-x-2">
                <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce"></div>
                <div
                  className="w-2 h-2 bg-gray-400 rounded-full animate-bounce"
                  style={{ animationDelay: "0.1s" }}
                ></div>
                <div
                  className="w-2 h-2 bg-gray-400 rounded-full animate-bounce"
                  style={{ animationDelay: "0.2s" }}
                ></div>
              </div>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className="border-t border-gray-800 p-4">
        <div className="flex space-x-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder={
              selectedModel
                ? "Type a message... (Enter to send, Shift+Enter for new line)"
                : "Load a model first..."
            }
            disabled={!selectedModel || loading}
            className="flex-1 bg-gray-800 border border-gray-700 rounded-lg p-3 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
            rows={3}
          />
          <button
            onClick={handleSend}
            disabled={!selectedModel || loading || !input.trim()}
            className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}
