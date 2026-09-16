import rust from './build/worker/shim.mjs';
import { createWorkerHandler } from './access.mjs';

export default createWorkerHandler(rust);
