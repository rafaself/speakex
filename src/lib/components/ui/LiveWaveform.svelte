<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  export let active = false;
  export let frozen = false;
  export let shimmer = false;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null;
  let animationId: number;
  let audioContext: AudioContext;
  let analyser: AnalyserNode;
  let dataArray: Uint8Array;
  let stream: MediaStream | null = null;

  // Waveform data: array of amplitudes [0, 1]
  let amplitudes: number[] = Array(100).fill(0.02);
  const maxBars = 100;
  let lastTime = 0;
  const sampleInterval = 50; // ms between samples

  $: if (active && !frozen && !stream) {
    void startMicrophone();
  } else if (!active && stream && !frozen) {
    stopMicrophone();
  }

  async function startMicrophone() {
    try {
      console.log("LiveWaveform: Starting microphone...");
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      audioContext = new AudioContext();
      
      if (audioContext.state === "suspended") {
        await audioContext.resume();
      }

      const source = audioContext.createMediaStreamSource(stream);
      analyser = audioContext.createAnalyser();
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.4;
      source.connect(analyser);
      dataArray = new Uint8Array(analyser.frequencyBinCount);
      
      console.log("LiveWaveform: Audio setup complete", {
        sampleRate: audioContext.sampleRate,
        fftSize: analyser.fftSize
      });

      if (animationId) cancelAnimationFrame(animationId);
      animationId = requestAnimationFrame(draw);
    } catch (err) {
      console.error("LiveWaveform: Error accessing microphone:", err);
    }
  }

  function stopMicrophone() {
    if (stream) {
      stream.getTracks().forEach((track) => track.stop());
      stream = null;
    }
    if (audioContext && audioContext.state !== "closed") {
      void audioContext.close();
    }
    cancelAnimationFrame(animationId);
  }

  function draw(time: number) {
    if (!canvas || !ctx) return;

    if (active && !frozen && analyser) {
      if (time - lastTime > sampleInterval) {
        analyser.getByteTimeDomainData(dataArray);
        
        // Calculate peak amplitude in this window
        let maxVal = 0;
        for (let i = 0; i < dataArray.length; i++) {
          const val = Math.abs(dataArray[i] - 128);
          if (val > maxVal) maxVal = val;
        }
        
        // Normalize 0-1 (max possible maxVal is 128)
        // Multiply by 1.5 for better sensitivity
        const normalized = Math.min(1, Math.max(0.02, (maxVal / 128) * 1.5));
        
        amplitudes = [...amplitudes.slice(1), normalized];
        lastTime = time;
      }
    }

    renderWaveform(time);
    animationId = requestAnimationFrame(draw);
  }

  function renderWaveform(time: number) {
    if (!ctx || !canvas) return;
    
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.width / dpr;
    const height = canvas.height / dpr;
    
    ctx.clearRect(0, 0, width, height);

    const barWidth = width / maxBars;
    const gap = 2;
    const actualBarWidth = Math.max(1, barWidth - gap);

    amplitudes.forEach((amp, i) => {
      const x = i * barWidth;
      // Loudest parts are taller, but even silence has a tiny height
      const barHeight = Math.max(4, amp * height * 0.8);
      const y = (height - barHeight) / 2;

      // Draw bar with a nice gradient or color
      if (shimmer) {
        ctx!.fillStyle = getShimmerColor(time, x);
      } else {
        // Subtle gradient for premium look
        const gradient = ctx!.createLinearGradient(x, y, x, y + barHeight);
        gradient.addColorStop(0, "rgba(255, 255, 255, 0.9)");
        gradient.addColorStop(0.5, "rgba(255, 255, 255, 1)");
        gradient.addColorStop(1, "rgba(255, 255, 255, 0.9)");
        ctx!.fillStyle = gradient;
      }
      
      // Use roundRect for modern look
      ctx!.beginPath();
      // @ts-expect-error roundRect is available in modern browsers
      if (ctx!.roundRect) {
        // @ts-expect-error roundRect is available in modern browsers
        ctx!.roundRect(x, y, actualBarWidth, barHeight, actualBarWidth / 2);
      } else {
        ctx!.rect(x, y, actualBarWidth, barHeight);
      }
      ctx!.fill();
    });
  }

  function getShimmerColor(time: number, x: number) {
    const shimmerSpeed = 0.003;
    const phase = (time * shimmerSpeed - x * 0.005) % 2;
    const intensity = 0.3 + 0.7 * (0.5 + 0.5 * Math.sin(phase * Math.PI));
    return `rgba(255, 255, 255, ${intensity})`;
  }

  onMount(() => {
    ctx = canvas.getContext("2d");
    const resize = () => {
      if (!canvas) return;
      const dpr = window.devicePixelRatio || 1;
      const rect = canvas.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      ctx?.scale(dpr, dpr);
    };
    resize();
    window.addEventListener("resize", resize);
    
    // Start animation loop even if not active (to handle shimmer/frozen)
    animationId = requestAnimationFrame(draw);

    return () => {
      window.removeEventListener("resize", resize);
      stopMicrophone();
    };
  });

  onDestroy(stopMicrophone);
</script>

<div class="waveform-container">
  <canvas bind:this={canvas} />
</div>

<style>
  .waveform-container {
    width: 100%;
    height: 48px;
    display: flex;
    align-items: center;
    overflow: hidden;
  }

  canvas {
    width: 100%;
    height: 100%;
  }
</style>
