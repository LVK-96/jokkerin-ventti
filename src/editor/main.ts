import init, {
    init_gpu,
    render_frame,
    resize_gpu,
    update_time_uniform,
    load_animation,
    enter_editor_mode,
    get_animation_keyframe_count,
    get_current_keyframe_time,
    set_editor_keyframe,
    remove_keyframe,
    add_keyframe_copy,
    get_joint_screen_positions,
    apply_joint_drag,
    export_animation_json,
    get_joint_info,
    set_joint_rotation,
    set_joint_position_editor,
    set_exercise,
    update_skeleton,
} from "../../wasm/pkg/jokkerin_ventti_wasm";
import { initCameraControls, updateCameraFromInput, setCameraEnabled, getCameraState } from '../camera';
import { animationMap } from '../animations';

// --- State ---
let currentExerciseName = '';
let currentKeyframe = 0;
let isDragging = false;
let selectedJoint: number | null = null;
let hoveredJoint: number | null = null;
let dragStartX = 0;
let dragStartY = 0;
let lastTime = 0;

// Undo/Redo
interface UndoState {
    keyframeIndex: number;
    poseJson: string;
    selectedJoint: number | null;
}
const undoStack: UndoState[] = [];
const redoStack: UndoState[] = [];
const MAX_UNDO_STATES = 10;


const JOINT_NAMES = [
    'Hips', 'Neck', 'Neck (Top)', 'Head',
    'Left Shoulder', 'Left Upper Arm', 'Left Forearm',
    'Right Shoulder', 'Right Upper Arm', 'Right Forearm',
    'Left Thigh', 'Left Shin',
    'Right Thigh', 'Right Shin'
];

// --- Initialization ---

async function initEditor() {
    await init();
    try {
        await init_gpu('gpu-canvas');
        console.log('WebGPU initialized');

        // Initialize camera
        const canvas = document.getElementById('gpu-canvas') as HTMLCanvasElement;
        const overlayCanvas = document.getElementById('overlay-canvas') as HTMLCanvasElement;
        initCameraControls(canvas);
        initOverlay(overlayCanvas);

        // Bind events
        bindEvents(canvas);

        // Populate exercise selector
        populateExerciseSelect();

        // Start loop
        requestAnimationFrame(animate);

        // Handle resize
        window.addEventListener('resize', () => resize_gpu('gpu-canvas'));

    } catch (e) {
        console.error('Failed to init WebGPU:', e);
    }
}

function populateExerciseSelect() {
    const select = document.getElementById('exercise-select') as HTMLSelectElement;
    if (!select) return;

    // Clear existing options except first
    while (select.options.length > 1) {
        select.remove(1);
    }

    for (const name of animationMap.keys()) {
        const option = document.createElement('option');
        option.value = name;
        option.textContent = name;
        select.appendChild(option);
    }

    select.addEventListener('change', (e) => {
        const name = (e.target as HTMLSelectElement).value;
        if (name) loadExercise(name);
    });
}

function loadExercise(name: string) {
    if (!animationMap.has(name)) return;

    currentExerciseName = name;
    const json = animationMap.get(name)!;

    set_exercise(name);
    load_animation(json);
    enter_editor_mode();

    currentKeyframe = 0;
    selectedJoint = null;
    undoStack.length = 0;
    redoStack.length = 0;

    updateUI();
}

// --- Animation Loop ---

function animate(time: number) {
    const delta = time - lastTime;
    lastTime = time;

    update_time_uniform(delta);
    updateCameraFromInput();
    update_skeleton(); // Renders editor pose because we called enter_editor_mode()
    update_time_uniform(delta);
    updateCameraFromInput();
    update_skeleton(); // Renders editor pose because we called enter_editor_mode()
    render_frame();
    drawOverlay();

    if (selectedJoint !== null) {
        // Update Info Panel
        updateJointUI(selectedJoint);
    } else {
        const ph = document.getElementById('joint-placeholder');
        const ctrl = document.getElementById('joint-controls');
        if (ph) ph.style.display = 'block';
        if (ctrl) ctrl.style.display = 'none';
    }

    requestAnimationFrame(animate);
}

// --- Interaction ---

let overlayCtx: CanvasRenderingContext2D | null = null;
let overlayCanvasRef: HTMLCanvasElement | null = null;

function initOverlay(canvas: HTMLCanvasElement) {
    overlayCanvasRef = canvas;
    overlayCtx = canvas.getContext('2d');
    resizeOverlay();
    window.addEventListener('resize', resizeOverlay);
}

function resizeOverlay() {
    if (!overlayCanvasRef) return;
    const rect = overlayCanvasRef.parentElement?.getBoundingClientRect();
    if (rect) {
        overlayCanvasRef.width = rect.width;
        overlayCanvasRef.height = rect.height;
    }
}

function drawOverlay() {
    if (!overlayCtx || !overlayCanvasRef) return;

    // Clear
    overlayCtx.clearRect(0, 0, overlayCanvasRef.width, overlayCanvasRef.height);

    drawGizmo(overlayCtx);

    // Get joint positions
    try {
        const positions = getLogicalJointPositions();
        if (!positions || positions.length === 0) return;

        // Draw joints
        for (let i = 0; i < positions.length / 2; i++) {
            const x = positions[i * 2];
            const y = positions[i * 2 + 1];

            // Skip if behind camera (-1000 scaled is still negative)
            if (x < -100) continue;

            const isHovered = i === hoveredJoint;
            const isSelected = i === selectedJoint;

            overlayCtx.beginPath();
            overlayCtx.arc(x, y, isSelected ? 8 : (isHovered ? 6 : 4), 0, Math.PI * 2);

            if (isSelected) {
                overlayCtx.fillStyle = '#ff0000';
                overlayCtx.strokeStyle = '#ffffff';
            } else if (isHovered) {
                overlayCtx.fillStyle = '#ffff00';
                overlayCtx.strokeStyle = '#000000';
            } else {
                overlayCtx.fillStyle = 'rgba(255, 255, 255, 0.7)';
                overlayCtx.strokeStyle = 'rgba(0, 0, 0, 0.5)';
            }

            overlayCtx.stroke();
        }

        drawGizmo(overlayCtx);
    } catch (e) { }
}

function bindEvents(canvas: HTMLCanvasElement) {
    // Mouse/Touch
    canvas.addEventListener('pointermove', onPointerMove);
    canvas.addEventListener('pointerdown', onPointerDown);
    canvas.addEventListener('pointerup', onPointerUp);

    // Timeline Buttons
    document.getElementById('prev-kf')?.addEventListener('click', () => navigateKeyframe(-1));
    document.getElementById('next-kf')?.addEventListener('click', () => navigateKeyframe(1));
    document.getElementById('add-kf')?.addEventListener('click', addKeyframe);
    document.getElementById('delete-kf')?.addEventListener('click', deleteKeyframe);
    document.getElementById('save-btn')?.addEventListener('click', copyJson);
    document.getElementById('undo-btn')?.addEventListener('click', undo);
    document.getElementById('redo-btn')?.addEventListener('click', redo);

    // Keyboard
    document.addEventListener('keydown', onKeyDown);

    // Joint Inputs
    bindJointInputs();
}

function bindJointInputs() {
    const inputs = ['j-x', 'j-y', 'j-z', 'j-rx', 'j-ry', 'j-rz'];
    inputs.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.addEventListener('change', onJointInputChange);
    });
}

function onJointInputChange(e: Event) {
    if (selectedJoint === null) return;
    const target = e.target as HTMLInputElement;
    const id = target.id;
    const val = parseFloat(target.value);

    // Validate input: reject empty or invalid values
    if (isNaN(val)) {
        // Restore the input to its current valid value from the model
        const infoJson = get_joint_info(selectedJoint);
        const info = JSON.parse(infoJson);
        if (id === 'j-x') target.value = info.x.toFixed(2);
        else if (id === 'j-y') target.value = info.y.toFixed(2);
        else if (id === 'j-z') target.value = info.z.toFixed(2);
        else if (id === 'j-rx') target.value = info.rx.toFixed(2);
        else if (id === 'j-ry') target.value = info.ry.toFixed(2);
        else if (id === 'j-rz') target.value = info.rz.toFixed(2);
        return;
    }


    // Get current values to mix
    const infoJson = get_joint_info(selectedJoint);
    const info = JSON.parse(infoJson);

    if (id.startsWith('j-r')) {
        // Rotation
        let rx = id === 'j-rx' ? val : info.rx;
        let ry = id === 'j-ry' ? val : info.ry;
        let rz = id === 'j-rz' ? val : info.rz;
        set_joint_rotation(selectedJoint, rx, ry, rz);
    } else {
        // Position
        let x = id === 'j-x' ? val : info.x;
        let y = id === 'j-y' ? val : info.y;
        let z = id === 'j-z' ? val : info.z;
        set_joint_position_editor(selectedJoint, x, y, z);
    }
}

function updateJointUI(jointId: number) {
    const infoJson = get_joint_info(jointId);
    const info = JSON.parse(infoJson);

    const ph = document.getElementById('joint-placeholder');
    const ctrl = document.getElementById('joint-controls');
    const idSpan = document.getElementById('j-id');

    if (ph) ph.style.display = 'none';
    if (ctrl) ctrl.style.display = 'block';
    if (idSpan) idSpan.textContent = `${jointId} (${JOINT_NAMES[jointId] || 'Unknown'})`;

    // Update inputs if not focused
    const setVal = (id: string, v: number) => {
        const el = document.getElementById(id) as HTMLInputElement;
        if (el && document.activeElement !== el) {
            el.value = v.toFixed(2);
        }
    };

    if (info.x !== undefined) {
        setVal('j-x', info.x);
        setVal('j-y', info.y);
        setVal('j-z', info.z);
        setVal('j-rx', info.rx);
        setVal('j-ry', info.ry);
        setVal('j-rz', info.rz);
    }
}

function onPointerMove(e: PointerEvent) {
    if (!currentExerciseName) return;

    // Update hovered joint
    const rect = (e.target as HTMLElement).getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (!isDragging) {
        hoveredJoint = findNearestJoint(x, y);
        // Change cursor
        (e.target as HTMLElement).style.cursor = hoveredJoint !== null ? 'pointer' : 'default';
        updateUI();
    }

    if (isDragging && selectedJoint !== null) {
        // Drag logic uses raw client deltas for now
        const dx = e.clientX - dragStartX;
        const dy = e.clientY - dragStartY;

        // Scale delta by DPR for WASM (which expects physical pixels)
        const dpr = window.devicePixelRatio || 1;
        apply_joint_drag(selectedJoint, dx * dpr, dy * dpr);

        dragStartX = e.clientX;
        dragStartY = e.clientY;
    }
}

function onPointerDown(e: PointerEvent) {
    if (!currentExerciseName) return;

    const canvas = e.target as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const joint = findNearestJoint(x, y);
    if (joint !== null) {
        saveUndoState();
        selectedJoint = joint;
        isDragging = true;
        dragStartX = e.clientX;
        dragStartY = e.clientY;

        setCameraEnabled(false);
        canvas.setPointerCapture(e.pointerId);

        updateUI();
    } else {
        selectedJoint = null;
        setCameraEnabled(true);
        updateUI();
    }
}

function onPointerUp(e: PointerEvent) {
    isDragging = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
}

function onKeyDown(e: KeyboardEvent) {
    // Shortcuts
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;

    if (e.key === '[') navigateKeyframe(-1);
    if (e.key === ']') navigateKeyframe(1);

    if (e.ctrlKey && e.key === 'z') undo();
    if (e.ctrlKey && (e.key === 'y' || (e.shiftKey && e.key === 'z'))) redo();

    if (e.key === 'Delete') deleteKeyframe();
}

// --- Logic ---

// --- Logic ---

function getLogicalJointPositions(): Float32Array | number[] | null {
    const raw = get_joint_screen_positions();
    if (!raw || raw.length === 0) return null;

    // Scale down by DPR if needed
    // WASM returns physical pixels (based on canvas.width/height)
    // We want logical CSS pixels for overlay and mouse interaction
    const dpr = window.devicePixelRatio || 1;
    if (dpr === 1) return raw;

    const scaled = new Float32Array(raw.length);
    for (let i = 0; i < raw.length; i++) {
        scaled[i] = raw[i] / dpr;
    }
    return scaled;
}

function findNearestJoint(sx: number, sy: number): number | null {
    try {
        const positions = getLogicalJointPositions();
        if (!positions || positions.length === 0) return null;

        let nearest: number | null = null;
        let minInfo = 20; // radius

        for (let i = 0; i < positions.length / 2; i++) {
            const jx = positions[i * 2];
            const jy = positions[i * 2 + 1];
            const d = Math.sqrt((jx - sx) ** 2 + (jy - sy) ** 2);
            if (d < minInfo) {
                minInfo = d;
                nearest = i;
            }
        }
        return nearest;
    } catch { return null; }
}

function navigateKeyframe(delta: number) {
    const count = get_animation_keyframe_count();
    if (count === 0) return;

    currentKeyframe = Math.max(0, Math.min(count - 1, currentKeyframe + delta));
    set_editor_keyframe(currentKeyframe);
    updateUI();
}

function addKeyframe() {
    saveUndoState();
    add_keyframe_copy(currentKeyframe);
    currentKeyframe++;
    set_editor_keyframe(currentKeyframe);
    updateUI();
}

function deleteKeyframe() {
    saveUndoState();
    remove_keyframe(currentKeyframe);
    const count = get_animation_keyframe_count();
    // Adjust index if needed
    if (currentKeyframe >= count) currentKeyframe = Math.max(0, count - 1);

    set_editor_keyframe(currentKeyframe);
    updateUI();
}

function copyJson() {
    try {
        const json = export_animation_json();
        navigator.clipboard.writeText(json);
        const btn = document.getElementById('save-btn');
        if (btn) {
            const old = btn.textContent;
            btn.textContent = '✅ Copied!';
            setTimeout(() => btn.textContent = old, 2000);
        }
    } catch (e) {
        console.error(e);
        alert('Failed to copy');
    }
}

// --- Undo/Redo ---

function saveUndoState() {
    try {
        const json = export_animation_json();
        undoStack.push({ keyframeIndex: currentKeyframe, poseJson: json, selectedJoint });
        if (undoStack.length > MAX_UNDO_STATES) undoStack.shift();
        redoStack.length = 0;
    } catch { }
}

function undo() {
    if (!undoStack.length) return;
    try {
        const currentHook = export_animation_json();
        redoStack.push({ keyframeIndex: currentKeyframe, poseJson: currentHook, selectedJoint });

        const state = undoStack.pop()!;
        load_animation(state.poseJson);
        enter_editor_mode();
        currentKeyframe = state.keyframeIndex;
        set_editor_keyframe(currentKeyframe);
        selectedJoint = state.selectedJoint;

        updateUI();
    } catch { }
}

function redo() {
    if (!redoStack.length) return;
    try {
        const currentHook = export_animation_json();
        undoStack.push({ keyframeIndex: currentKeyframe, poseJson: currentHook, selectedJoint });

        const state = redoStack.pop()!;
        load_animation(state.poseJson);
        enter_editor_mode();
        currentKeyframe = state.keyframeIndex;
        set_editor_keyframe(currentKeyframe);
        selectedJoint = state.selectedJoint;

        updateUI();
    } catch { }
}

// --- UI Updates ---

function updateUI() {
    const count = get_animation_keyframe_count();
    const time = get_current_keyframe_time();

    const timeDisplay = document.getElementById('time-display');
    if (timeDisplay) timeDisplay.textContent = `${time.toFixed(2)} s(Frame ${currentKeyframe + 1}/${count})`;

    const statusText = document.getElementById('status-text');
    if (statusText) {
        statusText.textContent = selectedJoint !== null
            ? `Selected: ${JOINT_NAMES[selectedJoint]} `
            : (count > 0 ? 'Ready' : 'No Animation');
    }
}

// Start
initEditor();

// --- Gizmo ---
function drawGizmo(ctx: CanvasRenderingContext2D) {
    console.log("Drawing Gizmo");
    ctx.fillStyle = 'purple'; ctx.fillRect(10, 10, 20, 20); // Test square

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
