export * from './logger';

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

export function formatSpeed(bytesPerSecond: number): string {
  return formatBytes(bytesPerSecond) + '/s';
}

export function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export function sanitizePath(filePath: string): string {
  // Remove any path traversal attempts
  return filePath.replace(/\.\./g, '').replace(/[<>:"|?*]/g, '');
}

export function parseModelId(url: string): { repo: string; filename: string; quantization?: string } | null {
  // Parse Hugging Face model URLs
  // Format: https://huggingface.co/author/model-name/resolve/main/filename.gguf
  // Or: author/model-name

  try {
    if (url.includes('huggingface.co')) {
      const match = url.match(/huggingface\.co\/([^/]+\/[^/]+)(?:\/resolve\/[^/]+\/(.+))?/);
      if (match) {
        return {
          repo: match[1],
          filename: match[2] || '',
        };
      }
    } else {
      // Simple format: author/model-name or author/model-name@quantization
      const [repo, quant] = url.split('@');
      return {
        repo,
        filename: '',
        quantization: quant,
      };
    }
  } catch (error) {
    return null;
  }

  return null;
}

export function estimateTokens(text: string): number {
  // Rough estimation: ~4 characters per token
  return Math.ceil(text.length / 4);
}

export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength - 3) + '...';
}

export async function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

export function debounce<T extends (...args: any[]) => any>(
  func: T,
  wait: number
): (...args: Parameters<T>) => void {
  let timeout: NodeJS.Timeout | null = null;

  return (...args: Parameters<T>) => {
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => func(...args), wait);
  };
}

export function throttle<T extends (...args: any[]) => any>(
  func: T,
  limit: number
): (...args: Parameters<T>) => void {
  let inThrottle: boolean;

  return (...args: Parameters<T>) => {
    if (!inThrottle) {
      func(...args);
      inThrottle = true;
      setTimeout(() => (inThrottle = false), limit);
    }
  };
}
