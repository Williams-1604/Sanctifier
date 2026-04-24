const SUPPORTED_CONTRACT_EXTENSIONS = new Set([".rs"]);
const MAX_CONTRACT_UPLOAD_SIZE_BYTES = 250 * 1024;

function getFileExtension(name: string): string {
  const idx = name.lastIndexOf(".");
  return idx >= 0 ? name.slice(idx).toLowerCase() : "";
}

export function validateContractUpload(file: File): string | null {
  if (file.size > MAX_CONTRACT_UPLOAD_SIZE_BYTES) {
    return `File size exceeds ${MAX_CONTRACT_UPLOAD_SIZE_BYTES / 1024} KB.`;
  }

  const extension = getFileExtension(file.name);
  if (!SUPPORTED_CONTRACT_EXTENSIONS.has(extension)) {
    return "Only .rs contract source files are supported.";
  }

  return null;
}
