// Type declarations for web-haptics (haptic feedback library)
// See: https://github.com/lochie/web-haptics

declare module "web-haptics" {
  export type HapticPresetName = "success" | "nudge" | "error" | "buzz";

  export interface Vibration {
    duration: number;
    delay?: number;
    intensity?: number;
  }

  export interface HapticPreset {
    pattern: Vibration[];
    description?: string;
  }

  export interface WebHapticsOptions {
    debug?: boolean;
    showSwitch?: boolean;
  }

  export class WebHaptics {
    constructor(options?: WebHapticsOptions);
    trigger(
      input?: HapticPresetName | number | number[] | Vibration[] | HapticPreset,
      options?: { intensity?: number },
    ): Promise<void>;
    cancel(): void;
    destroy(): void;
    setDebug(debug: boolean): void;
    setShowSwitch(show: boolean): void;
    static isSupported: boolean;
  }
}

declare module "web-haptics/react" {
  export function useWebHaptics(): {
    trigger: (
      input?:
        | "success"
        | "nudge"
        | "error"
        | "buzz"
        | number
        | number[]
        | Array<{ duration: number; delay?: number; intensity?: number }>
        | {
            pattern: Array<{
              duration: number;
              delay?: number;
              intensity?: number;
            }>;
            description?: string;
          },
      options?: { intensity?: number },
    ) => Promise<void>;
    cancel: () => void;
  };
}
