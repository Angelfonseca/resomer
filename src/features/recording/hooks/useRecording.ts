import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type RecordingSource = 'microphone' | 'system_audio' | 'both';
export type RecordingState = 'idle' | 'recording' | 'paused' | 'stopped';

export interface AudioDevice {
  name: string;
  isDefault: boolean;
  channels: number;
  sampleRates: number[];
}

interface Recording {
  meetingId: string;
  state: RecordingState;
  source: RecordingSource;
  filePath?: string;
  durationMs: number;
  startedAt?: string;
  stoppedAt?: string;
}

export const useAudioDevices = () => {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchDevices = async () => {
      try {
        const result = await invoke<AudioDevice[]>('list_audio_devices');
        setDevices(result);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        // Fallback to default options if device enumeration fails
        setDevices([
          { name: 'Default Microphone', isDefault: true, channels: 1, sampleRates: [16000, 48000] },
        ]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchDevices();
  }, []);

  return { devices, isLoading, error };
};

export const useRecording = () => {
  const [recording, setRecording] = useState<Recording | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startRecording = useCallback(
    async (meetingId: string, outputPath: string, source: RecordingSource = 'microphone') => {
      setIsLoading(true);
      setError(null);
      try {
        // El backend resuelve la ruta a una ubicación absoluta escribible
        // (~/.resomer/recordings) y la devuelve; la usamos para el pipeline.
        const resolvedPath = await invoke<string>('start_recording', {
          meetingId,
          outputPath,
          source,
        });

        setRecording({
          meetingId,
          state: 'recording',
          source,
          filePath: resolvedPath,
          durationMs: 0,
          startedAt: new Date().toISOString(),
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
      } finally {
        setIsLoading(false);
      }
    },
    []
  );

  const stopRecording = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      if (recording) {
        await invoke('stop_recording', { meetingId: recording.meetingId });
        setRecording({
          ...recording,
          state: 'stopped',
          stoppedAt: new Date().toISOString(),
        });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [recording]);

  const pauseRecording = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      await invoke('pause_recording');

      if (recording) {
        setRecording({
          ...recording,
          state: 'paused',
        });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [recording]);

  return {
    recording,
    isLoading,
    error,
    startRecording,
    stopRecording,
    pauseRecording,
  };
};
