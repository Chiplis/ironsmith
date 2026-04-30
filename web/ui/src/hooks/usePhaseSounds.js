import { useEffect, useRef } from "react";
import { normalizePhaseKey, normalizeStepKey } from "@/lib/constants";

let audioContext = null;
let audioUnlocked = false;

function currentAudioTime() {
  return audioContext?.currentTime || 0;
}

function getAudioContext() {
  if (typeof window === "undefined") return null;
  const AudioCtor = window.AudioContext || window.webkitAudioContext;
  if (!AudioCtor) return null;
  if (!audioContext) {
    audioContext = new AudioCtor();
  }
  return audioContext;
}

function scheduleGain(gainNode, start, peak, end, endValue = 0.0001) {
  gainNode.gain.cancelScheduledValues(start);
  gainNode.gain.setValueAtTime(0.0001, start);
  gainNode.gain.exponentialRampToValueAtTime(Math.max(peak, 0.0001), start + 0.018);
  gainNode.gain.exponentialRampToValueAtTime(endValue, end);
}

function playTone({ frequency, endFrequency = frequency, type = "sine", start, duration, gain = 0.08, destination }) {
  const ctx = getAudioContext();
  if (!ctx || !destination) return;

  const osc = ctx.createOscillator();
  const amp = ctx.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(frequency, start);
  if (endFrequency !== frequency) {
    osc.frequency.exponentialRampToValueAtTime(Math.max(endFrequency, 20), start + duration);
  }
  scheduleGain(amp, start, gain, start + duration);
  osc.connect(amp).connect(destination);
  osc.start(start);
  osc.stop(start + duration + 0.03);
}

function playNoise({ start, duration, gain = 0.05, destination, filterType = "bandpass", frequency = 1200, q = 0.9 }) {
  const ctx = getAudioContext();
  if (!ctx || !destination) return;

  const buffer = ctx.createBuffer(1, Math.max(1, Math.floor(ctx.sampleRate * duration)), ctx.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < data.length; i += 1) {
    data[i] = (Math.random() * 2 - 1) * (1 - i / data.length);
  }

  const source = ctx.createBufferSource();
  const filter = ctx.createBiquadFilter();
  const amp = ctx.createGain();
  source.buffer = buffer;
  filter.type = filterType;
  filter.frequency.setValueAtTime(frequency, start);
  filter.Q.setValueAtTime(q, start);
  scheduleGain(amp, start, gain, start + duration, 0.0001);
  source.connect(filter).connect(amp).connect(destination);
  source.start(start);
}

function createMaster() {
  const ctx = getAudioContext();
  if (!ctx) return null;
  const master = ctx.createGain();
  master.gain.setValueAtTime(0.34, ctx.currentTime);
  master.connect(ctx.destination);
  return master;
}

function playDrawSound() {
  const ctx = getAudioContext();
  const master = createMaster();
  if (!ctx || !master) return;
  const now = currentAudioTime();
  playNoise({ start: now, duration: 0.13, gain: 0.035, destination: master, filterType: "highpass", frequency: 900 });
  playNoise({ start: now + 0.07, duration: 0.12, gain: 0.026, destination: master, filterType: "bandpass", frequency: 1700, q: 0.7 });
  playTone({ frequency: 520, endFrequency: 760, type: "triangle", start: now + 0.015, duration: 0.09, gain: 0.018, destination: master });
}

function playGongSound() {
  const ctx = getAudioContext();
  const master = createMaster();
  if (!ctx || !master) return;
  const now = currentAudioTime();
  playTone({ frequency: 164, endFrequency: 112, type: "sine", start: now, duration: 0.58, gain: 0.09, destination: master });
  playTone({ frequency: 246, endFrequency: 221, type: "triangle", start: now + 0.006, duration: 0.44, gain: 0.026, destination: master });
  playTone({ frequency: 492, endFrequency: 466, type: "sine", start: now + 0.012, duration: 0.28, gain: 0.012, destination: master });
}

function playSwordSound() {
  const ctx = getAudioContext();
  const master = createMaster();
  if (!ctx || !master) return;
  const now = currentAudioTime();
  playNoise({ start: now, duration: 0.16, gain: 0.026, destination: master, filterType: "highpass", frequency: 2200 });
  playTone({ frequency: 740, endFrequency: 1640, type: "sawtooth", start: now + 0.018, duration: 0.18, gain: 0.028, destination: master });
  playTone({ frequency: 1840, endFrequency: 2320, type: "triangle", start: now + 0.09, duration: 0.11, gain: 0.015, destination: master });
}

function playShieldSound() {
  const ctx = getAudioContext();
  const master = createMaster();
  if (!ctx || !master) return;
  const now = currentAudioTime();
  playNoise({ start: now, duration: 0.12, gain: 0.055, destination: master, filterType: "bandpass", frequency: 720, q: 1.6 });
  playTone({ frequency: 238, endFrequency: 188, type: "square", start: now, duration: 0.22, gain: 0.045, destination: master });
  playTone({ frequency: 640, endFrequency: 410, type: "triangle", start: now + 0.012, duration: 0.24, gain: 0.026, destination: master });
}

function phaseSoundKey(phase, step) {
  const normalizedStep = normalizeStepKey(step);
  const normalizedPhase = normalizePhaseKey(phase);

  if (normalizedStep === "Draw") return "draw";
  if (normalizedStep === "DeclareAttackers") return "attackers";
  if (normalizedStep === "DeclareBlockers") return "blockers";
  if (normalizedStep === "End" || normalizedStep === "Cleanup") return "gong";
  if (normalizedPhase === "FirstMain" || normalizedPhase === "NextMain" || normalizedPhase === "Ending") return "gong";
  return null;
}

function playPhaseSound(key) {
  if (!audioUnlocked) return;
  const ctx = getAudioContext();
  if (!ctx) return;
  ctx.resume?.().catch(() => {});

  switch (key) {
    case "draw":
      playDrawSound();
      break;
    case "attackers":
      playSwordSound();
      break;
    case "blockers":
      playShieldSound();
      break;
    case "gong":
      playGongSound();
      break;
    default:
      break;
  }
}

function unlockAudio() {
  const ctx = getAudioContext();
  if (!ctx) return;
  ctx.resume?.().catch(() => {});
  audioUnlocked = true;
}

export default function usePhaseSounds(state) {
  const previousSignatureRef = useRef("");

  useEffect(() => {
    if (typeof window === "undefined") return undefined;
    const options = { once: true, passive: true };
    window.addEventListener("pointerdown", unlockAudio, options);
    window.addEventListener("keydown", unlockAudio, { once: true });
    window.addEventListener("touchstart", unlockAudio, options);
    return () => {
      window.removeEventListener("pointerdown", unlockAudio, options);
      window.removeEventListener("keydown", unlockAudio, { once: true });
      window.removeEventListener("touchstart", unlockAudio, options);
    };
  }, []);

  useEffect(() => {
    if (!state) return;
    const signature = [
      state.turn_number ?? "",
      state.phase ?? "",
      state.step ?? "",
    ].join("|");
    if (!signature || signature === previousSignatureRef.current) return;

    const previousSignature = previousSignatureRef.current;
    previousSignatureRef.current = signature;
    if (!previousSignature) return;

    const key = phaseSoundKey(state.phase, state.step);
    if (key) playPhaseSound(key);
  }, [state?.turn_number, state?.phase, state?.step, state]);
}
