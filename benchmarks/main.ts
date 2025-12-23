import init, { run_benchmarks } from '../wasm/pkg/jokkerin_ventti_wasm';

async function main() {
    await init();

    const runBtn = document.getElementById('run-btn') as HTMLButtonElement;
    const resultsDiv = document.getElementById('results') as HTMLDivElement;
    const iterationsInput = document.getElementById('iterations') as HTMLInputElement;

    runBtn.addEventListener('click', () => {
        const iterations = parseInt(iterationsInput.value) || 1000000;

        runBtn.disabled = true;
        resultsDiv.textContent = `Running ${iterations.toLocaleString()} iterations...`;

        // Small timeout to allow UI to update
        setTimeout(() => {
            try {
                const res = run_benchmarks(iterations);

                let output = `Results for ${res.iterations.toLocaleString()} iterations:\n\n`;

                output += `\n--- Matrix Multiply (4x4) * (4x4) ---\n`;
                output += `1. Scalar:      ${res.scalar_ms.toFixed(2)}ms (Baseline)\n`;

                if (res.simd_std_ms != null) {
                    const simdSpeedup = (res.scalar_ms / res.simd_std_ms).toFixed(1);
                    output += `2. Portable SIMD:     ${res.simd_std_ms.toFixed(2)}ms (${simdSpeedup}x faster)\n`;
                } else {
                    output += `2. Portable SIMD:     Not Enabled (requires nightly)\n`;
                }

                if (res.fma_supported) {
                    const fmaSpeedup = (res.scalar_ms / res.fma_relaxed_ms).toFixed(1);
                    output += `3. Relaxed SIMD:     ${res.fma_relaxed_ms.toFixed(2)}ms (${fmaSpeedup}x faster)\n`;
                } else {
                    output += `3. Relaxed SIMD:     Not Supported in this browser\n`;
                }

                const glamSpeedup = (res.scalar_ms / res.glam_ms).toFixed(1);
                output += `4. Glam:     ${res.glam_ms.toFixed(2)}ms (${glamSpeedup}x faster)\n`;

                output += `\n--- Transpose (4x4) ---\n`;
                output += `1. Scalar:     ${res.transpose_scalar_ms.toFixed(2)}ms (Baseline)\n`;

                if (res.transpose_std_ms != null) {
                    const tSimdSpeedup = (res.transpose_scalar_ms / res.transpose_std_ms).toFixed(1);
                    output += `2. Portable SIMD:          ${res.transpose_std_ms.toFixed(2)}ms (${tSimdSpeedup}x faster)\n`;
                } else {
                    output += `2. Portable SIMD:          Not Enabled (requires nightly)\n`;
                }

                if (res.transpose_relaxed_ms != null) {
                    const tRelaxedSpeedup = (res.transpose_scalar_ms / res.transpose_relaxed_ms).toFixed(1);
                    output += `3. Relaxed SIMD:     ${res.transpose_relaxed_ms.toFixed(2)}ms (${tRelaxedSpeedup}x faster)\n`;
                }

                const tGlamSpeedup = (res.transpose_scalar_ms / res.transpose_glam_ms).toFixed(1);
                output += `4. Glam:     ${res.transpose_glam_ms.toFixed(2)}ms (${tGlamSpeedup}x faster)\n`;

                resultsDiv.innerHTML = output.replace(/(\(\d+\.?\d*x faster\))/g, '<span class="speedup">$1</span>');
                console.table(res);
            } catch (e) {
                resultsDiv.textContent = `Error: ${e}`;
                console.error(e);
            } finally {
                runBtn.disabled = false;
            }
        }, 50);
    });
}

main();
