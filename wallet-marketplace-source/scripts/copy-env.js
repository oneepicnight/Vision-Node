import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const envExample = path.join(__dirname, '..', '.env.example');
const envLocal = path.join(__dirname, '..', '.env.local');

if (!fs.existsSync(envLocal) && fs.existsSync(envExample)) {
  fs.copyFileSync(envExample, envLocal);
  console.log('[env] .env.local created from .env.example');
} else if (fs.existsSync(envLocal)) {
  console.log('[env] .env.local already exists');
} else {
  console.warn('[env] Warning: .env.example not found');
}
