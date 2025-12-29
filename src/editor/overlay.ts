import { get_camera_view_matrix } from '../../wasm/pkg/jokkerin_ventti_wasm';

/**
 * Draw the gizmo showing world coordinate axes
 * Uses the camera view matrix from WASM to project axes to screen space
 */
export function drawGizmo(ctx: CanvasRenderingContext2D) {
    // Get view matrix from WASM (flattened 4x4, column-major)
    let viewMatrix: Float32Array;
    try {
        viewMatrix = get_camera_view_matrix();
    } catch {
        // GPU not initialized yet, skip gizmo
        return;
    }

    if (viewMatrix.length !== 16) return;

    // Extract the rotation part of the view matrix (upper-left 3x3)
    // View matrix transforms world to camera space
    // The columns are the world X, Y, Z axes in camera space
    // We need to project world axes onto screen (camera X = right, camera Y = up)

    // View matrix columns (column-major order):
    // col 0: right vector (camera X direction in world space, but transposed in view matrix)
    // Actually, view matrix rows are camera axes directions in world space
    // Row 0 = Right, Row 1 = Up, Row 2 = Forward (into screen for RH)

    // For a view matrix V, the upper 3x3 rotation R = V^T (for orthogonal rotation)
    // Screen coords: project world axis onto Right (row 0) and Up (row 1)

    // View matrix in column-major: [m00, m10, m20, m30, m01, m11, m21, m31, ...]
    // Row 0: m00, m01, m02 = indices 0, 4, 8 (right vector)
    // Row 1: m10, m11, m12 = indices 1, 5, 9 (up vector)

    const rx = viewMatrix[0], ry = viewMatrix[4], rz = viewMatrix[8];   // Right vector
    const ux = viewMatrix[1], uy = viewMatrix[5], uz = viewMatrix[9];   // Up vector

    // Gizmo origin (bottom-left corner)
    const originX = 50;
    const originY = ctx.canvas.height - 200;
    const axisLen = 40;

    // Project world axis (ax, ay, az) to screen coords
    const project = (ax: number, ay: number, az: number): [number, number] => {
        // Dot with right = screen X, dot with up = screen Y (inverted for canvas)
        const px = ax * rx + ay * ry + az * rz;
        const py = ax * ux + ay * uy + az * uz;
        return [originX + px * axisLen, originY - py * axisLen];
    };

    ctx.lineWidth = 3;
    ctx.font = '12px sans-serif';
    ctx.lineCap = 'round';

    // X Axis (Red)
    const [xx, xy] = project(1, 0, 0);
    ctx.beginPath(); ctx.moveTo(originX, originY); ctx.lineTo(xx, xy);
    ctx.strokeStyle = '#ff3333'; ctx.stroke();
    ctx.fillStyle = '#ff3333'; ctx.fillText('X', xx, xy);

    // Y Axis (Green)
    const [yx, yy] = project(0, 1, 0);
    ctx.beginPath(); ctx.moveTo(originX, originY); ctx.lineTo(yx, yy);
    ctx.strokeStyle = '#33ff33'; ctx.stroke();
    ctx.fillStyle = '#33ff33'; ctx.fillText('Y', yx, yy);

    // Z Axis (Blue)
    const [zx, zy] = project(0, 0, 1);
    ctx.beginPath(); ctx.moveTo(originX, originY); ctx.lineTo(zx, zy);
    ctx.strokeStyle = '#3366ff'; ctx.stroke();
    ctx.fillStyle = '#3366ff'; ctx.fillText('Z', zx, zy);
}
