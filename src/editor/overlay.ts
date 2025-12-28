import { getCameraState } from '../camera';

export function drawGizmo(ctx: CanvasRenderingContext2D) {
    // console.log("Drawing Gizmo");

    const { azimuth, elevation } = getCameraState();

    // View Basis Calculation
    // Camera Position C (spherical to cartesian)
    // We only care about rotation.
    // D (Direction from Camera to Target) is inward.
    // Target - Camera.
    // Let's assume Camera is at (sin(az)cos(el), sin(el), cos(az)cos(el))
    // Looking at (0,0,0).
    // Forward F = -CameraPos.normalized().

    const cosEl = Math.cos(elevation);
    const sinEl = Math.sin(elevation);

    // Camera Pos Direction (Unit)
    const cx = Math.sin(azimuth) * cosEl;
    const cy = sinEl;
    const cz = Math.cos(azimuth) * cosEl;

    // Forward Vector (Camera -> Target)
    // Note: In Three.js/OpenGL, Camera "Forward" is -Z (local).
    // But "View Direction" is Target - Eye.
    const fx = -cx;
    const fy = -cy;
    const fz = -cz;

    // World Up is (0,1,0)
    // Right = Cross(F, Up).Normalized
    // Right = (fy*0 - fz*1, fz*0 - fx*0, fx*1 - fy*0)
    //       = (-fz, 0, fx)
    let rx = -fz;
    let ry = 0;
    let rz = fx;

    // Normalize Right
    const rLen = Math.sqrt(rx * rx + rz * rz);
    if (rLen > 0.0001) {
        rx /= rLen; rz /= rLen;
    }

    // Up = Cross(Right, Forward)
    // Ux = ry*fz - rz*fy = 0 - rz*fy = -rz*fy
    // Uy = rz*fx - rx*fz
    // Uz = rx*fy - ry*fx = rx*fy

    const ux = -rz * fy;
    const uy = rz * fx - rx * fz;
    const uz = rx * fy;

    // Gizmo Center
    const originX = 50;
    const originY = ctx.canvas.height - 200; // Above legend (bottom bar 60 + legend ~100 + padding)
    const axisLen = 40;

    // Project Axes
    // Dot Product with Right (Screen X) and Up (Screen Y, inverted)
    // Screen X = Dot(Axis, Right)
    // Screen Y = -Dot(Axis, Up) (Since screen Y is down)
    // Correction:
    // If I pan camera right, object moves left.
    // If Right vector points Right on screen.
    // If P is (1,0,0). ProjX = Dot(P, Right).
    // If P is in direction of Right, it should be Positive X on screen.
    // Yes.

    const project = (ax: number, ay: number, az: number) => {
        const px = ax * rx + ay * ry + az * rz;
        const py = ax * ux + ay * uy + az * uz;
        return [originX + px * axisLen, originY - py * axisLen]; // Y inverted for canvas
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
