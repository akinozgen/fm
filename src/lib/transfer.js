import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function paste(srcPaths, destDir, op) {
  return invoke('paste_cmd', { srcPaths, destDir, op });
}

export function cancelTransfer(jobId) {
  return invoke('cancel_transfer_cmd', { job_id: jobId });
}

export function pauseTransfer(jobId) {
  return invoke('pause_transfer_cmd', { job_id: jobId });
}

export function resumeTransfer(jobId) {
  return invoke('resume_transfer_cmd', { job_id: jobId });
}

export function listenTransferProgress(cb) {
  return listen('fm://transfer-progress', (e) => cb(e.payload));
}

export function listenTransferDone(cb) {
  return listen('fm://transfer-done', (e) => cb(e.payload));
}
