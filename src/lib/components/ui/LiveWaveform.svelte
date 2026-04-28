<script lang="ts">
  import { onMount } from "svelte";

  export let active = false;
  export let frozen = false;
  export let shimmer = false;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null;
  let animationId: number;

  const maxBars = 100;
  let amplitudes: number[] = Array(maxBars).fill(0.02);
  let currentRight = 0.02;
  let targetRight = 0.02;
  let lastSampleTime = 0;
  const sampleInterval = 80;

  function draw(time: number) {
    if (!canvas || !ctx) return;

    if (active && !frozen) {
      if (time - lastSampleTime > sampleInterval) {
        amplitudes = [...amplitudes.slice(1), currentRight];
        targetRight = 0.05 + Math.random() * 0.85;
        lastSampleTime = time;
      }
      currentRight += (targetRight - currentRight) * 0.15;
      amplitudes[amplitudes.length - 1] = currentRight;
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
      const barHeight = Math.max(4, amp * height * 0.8);
      const y = (height - barHeight) / 2;

      if (shimmer) {
        ctx!.fillStyle = getShimmerColor(time, x);
      } else {
        const gradient = ctx!.createLinearGradient(x, y, x, y + barHeight);
        gradient.addColorStop(0, "rgba(255, 255, 255, 0.9)");
        gradient.addColorStop(0.5, "rgba(255, 255, 255, 1)");
        gradient.addColorStop(1, "rgba(255, 255, 255, 0.9)");
        ctx!.fillStyle = gradient;
      }

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

    animationId = requestAnimationFrame(draw);

    return () => {
      window.removeEventListener("resize", resize);
      cancelAnimationFrame(animationId);
    };
  });
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
