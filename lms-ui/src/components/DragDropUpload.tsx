import { useState, useCallback, useRef } from "react";

interface DragDropUploadProps {
  onFilesSelected: (files: File[]) => void;
  accept?: string[];
  maxSize?: number; // in bytes
  maxFiles?: number;
  disabled?: boolean;
  className?: string;
  children?: React.ReactNode;
}

interface FileError {
  file: File;
  error: string;
}

const DEFAULT_ACCEPT = [".pdf", ".docx", ".txt", ".md"];
const DEFAULT_MAX_SIZE = 50 * 1024 * 1024; // 50MB

export default function DragDropUpload({
  onFilesSelected,
  accept = DEFAULT_ACCEPT,
  maxSize = DEFAULT_MAX_SIZE,
  maxFiles = 10,
  disabled = false,
  className = "",
  children,
}: DragDropUploadProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [errors, setErrors] = useState<FileError[]>([]);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const dragCounter = useRef(0);

  const validateFile = useCallback(
    (file: File): string | null => {
      // Check file extension
      const ext = "." + file.name.split(".").pop()?.toLowerCase();
      if (!accept.includes(ext)) {
        return `File type ${ext} not supported. Allowed: ${accept.join(", ")}`;
      }

      // Check file size
      if (file.size > maxSize) {
        const sizeMB = (maxSize / (1024 * 1024)).toFixed(0);
        return `File too large. Maximum size: ${sizeMB}MB`;
      }

      return null;
    },
    [accept, maxSize]
  );

  const handleFiles = useCallback(
    (fileList: FileList | null) => {
      if (!fileList || disabled) return;

      const files = Array.from(fileList);
      const validFiles: File[] = [];
      const newErrors: FileError[] = [];

      // Limit number of files
      const filesToProcess = files.slice(0, maxFiles);

      for (const file of filesToProcess) {
        const error = validateFile(file);
        if (error) {
          newErrors.push({ file, error });
        } else {
          validFiles.push(file);
        }
      }

      if (files.length > maxFiles) {
        newErrors.push({
          file: files[maxFiles],
          error: `Maximum ${maxFiles} files allowed`,
        });
      }

      setErrors(newErrors);

      if (validFiles.length > 0) {
        onFilesSelected(validFiles);
      }
    },
    [validateFile, maxFiles, disabled, onFilesSelected]
  );

  const handleDragEnter = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      dragCounter.current++;

      if (!disabled && e.dataTransfer.items.length > 0) {
        setIsDragging(true);
      }
    },
    [disabled]
  );

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current--;

    if (dragCounter.current === 0) {
      setIsDragging(false);
    }
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragging(false);
      dragCounter.current = 0;

      handleFiles(e.dataTransfer.files);
    },
    [handleFiles]
  );

  const handleClick = useCallback(() => {
    if (!disabled) {
      fileInputRef.current?.click();
    }
  }, [disabled]);

  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      handleFiles(e.target.files);
      // Reset input so same file can be selected again
      e.target.value = "";
    },
    [handleFiles]
  );

  const clearErrors = useCallback(() => {
    setErrors([]);
  }, []);

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  };

  return (
    <div className={className}>
      <div
        onClick={handleClick}
        onDragEnter={handleDragEnter}
        onDragLeave={handleDragLeave}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
        className={`
          relative border-2 border-dashed rounded-lg p-8 text-center cursor-pointer
          transition-all duration-200 ease-in-out
          ${
            disabled
              ? "border-gray-700 bg-gray-800/30 cursor-not-allowed opacity-50"
              : isDragging
              ? "border-blue-500 bg-blue-500/10 scale-[1.02]"
              : "border-gray-600 hover:border-gray-500 hover:bg-gray-800/50"
          }
        `}
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept={accept.join(",")}
          onChange={handleInputChange}
          className="hidden"
          disabled={disabled}
        />

        {children || (
          <div className="space-y-3">
            <div className="flex justify-center">
              <svg
                className={`w-12 h-12 ${
                  isDragging ? "text-blue-500" : "text-gray-500"
                }`}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                />
              </svg>
            </div>

            <div>
              <p className="text-gray-300 font-medium">
                {isDragging
                  ? "Drop files here"
                  : "Drag and drop files here, or click to browse"}
              </p>
              <p className="text-gray-500 text-sm mt-1">
                Supports {accept.join(", ")} (max {formatFileSize(maxSize)})
              </p>
            </div>
          </div>
        )}

        {/* Drag overlay */}
        {isDragging && (
          <div className="absolute inset-0 bg-blue-500/5 rounded-lg pointer-events-none" />
        )}
      </div>

      {/* Error messages */}
      {errors.length > 0 && (
        <div className="mt-3 space-y-2">
          {errors.map((err, idx) => (
            <div
              key={idx}
              className="flex items-start gap-2 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-sm"
            >
              <svg
                className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <div className="flex-1">
                <p className="text-red-400 font-medium">{err.file.name}</p>
                <p className="text-red-400/80">{err.error}</p>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setErrors(errors.filter((_, i) => i !== idx));
                }}
                className="text-gray-500 hover:text-gray-300"
              >
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>
          ))}
          <button
            onClick={(e) => {
              e.stopPropagation();
              clearErrors();
            }}
            className="text-sm text-gray-500 hover:text-gray-300"
          >
            Clear all errors
          </button>
        </div>
      )}
    </div>
  );
}

/**
 * Hook for handling file drops on any element
 */
export function useFileDrop(
  onDrop: (files: File[]) => void,
  options: { accept?: string[]; disabled?: boolean } = {}
) {
  const [isDragging, setIsDragging] = useState(false);
  const dragCounter = useRef(0);

  const handlers = {
    onDragEnter: (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      dragCounter.current++;
      if (!options.disabled) {
        setIsDragging(true);
      }
    },
    onDragLeave: (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      dragCounter.current--;
      if (dragCounter.current === 0) {
        setIsDragging(false);
      }
    },
    onDragOver: (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
    },
    onDrop: (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragging(false);
      dragCounter.current = 0;

      if (!options.disabled && e.dataTransfer.files.length > 0) {
        const files = Array.from(e.dataTransfer.files);
        if (options.accept) {
          const filtered = files.filter((f) => {
            const ext = "." + f.name.split(".").pop()?.toLowerCase();
            return options.accept?.includes(ext);
          });
          onDrop(filtered);
        } else {
          onDrop(files);
        }
      }
    },
  };

  return { isDragging, handlers };
}
