import initWasm, {
    load_animation,
    // Singleton editor API
    start_editing,
    stop_editing,
    is_editing,
    get_keyframe_count,
    get_keyframe_time,
    set_keyframe_index,
    add_keyframe,
    delete_keyframe,
    get_joint_positions,
    drag_joint,
    export_clip_json,
    get_bone_info,
    set_bone_rotation,
    set_bone_position,
    set_exercise,
    update_skeleton_from_session,
    AnimationId
} from "../../wasm/pkg/jokkerin_ventti_wasm";
import { setCameraEnabled, getViewMatrix, getProjectionMatrix } from '../camera';
import { resolveAnimationId, animationData, DISPLAY_NAMES } from '../animations';
import { initHistory, saveUndoState, undo, redo, clearHistory } from './history';
import { drawGizmo } from './overlay';
import { WebGPUEngine } from '../engine';

// --- State ---
let currentExerciseName = '';
let currentExerciseId: AnimationId | null = null;
let currentKeyframe = 0;
let isDragging = false;
let selectedJoint: number | null = null;
let hoveredJoint: number | null = null;
let dragStartX = 0;
let dragStartY = 0;

const JOINT_NAMES = [
    'Hips', 'Neck', 'Neck (Top)', 'Head',
    'Left Shoulder', 'Left Upper Arm', 'Left Forearm',
    'Right Shoulder', 'Right Upper Arm', 'Right Forearm',
    'Left Thigh', 'Left Shin',
    'Right Thigh', 'Right Shin'
];

let engine: WebGPUEngine | null = null;

// --- Initialization ---

async function init() {
    await initWasm();
    try {
        engine = new WebGPUEngine('gpu-canvas');
        await engine.init();

        // Initialize history
        initHistory({
            getKeyframeIndex: () => currentKeyframe,
            getPoseJson: () => is_editing() ? export_clip_json() : '{}',
            getSelectedJoint: () => selectedJoint,
            loadPose: (json) => {
                // Destroy old session, create new one with modified clip
                stop_editing();
                if (currentExerciseId !== null) {
                    load_animation(currentExerciseId, json);
                    start_editing(currentExerciseId);
                }
            },
            setKeyframeIndex: (idx) => {
                currentKeyframe = idx;
                set_keyframe_index(idx);
            },
            setSelectedJoint: (idx) => {
                selectedJoint = idx;
            },
            onHistoryRestore: () => {
                updateUI();
            }
        });

        const canvas = document.getElementById('gpu-canvas') as HTMLCanvasElement;
        const overlayCanvas = document.getElementById('overlay-canvas') as HTMLCanvasElement;
        initOverlay(overlayCanvas);

        // Bind events
        bindEvents(canvas);

        // Populate exercise selector
        populateExerciseSelect();

        // Start loop via Engine
        engine.start(onEngineUpdate);

    } catch (e) {
        console.error('Failed to init WebGPU:', e);
    }
}

function onEngineUpdate(_delta: number) {
    // Use handle-based skeleton update if handle is valid
    if (is_editing()) {
        update_skeleton_from_session();
    }

    drawOverlay();

    if (selectedJoint !== null) {
        updateJointUI(selectedJoint);
    } else {
        const ph = document.getElementById('joint-placeholder');
        const ctrl = document.getElementById('joint-controls');
        if (ph) ph.style.display = 'block';
        if (ctrl) ctrl.style.display = 'none';
    }
}

function populateExerciseSelect() {
    const select = document.getElementById('exercise-select') as HTMLSelectElement;
    if (!select) return;

    // Clear existing options except first
    while (select.options.length > 1) {
        select.remove(1);
    }

    // Populate using DISPLAY_NAMES
    for (const displayName of Object.values(DISPLAY_NAMES)) {
        const option = document.createElement('option');
        option.value = displayName;
        option.textContent = displayName;
        select.appendChild(option);
    }

    select.addEventListener('change', (e) => {
        const name = (e.target as HTMLSelectElement).value;
        if (name) loadExercise(name);
    });
}

function loadExercise(name: string) {
    const id = resolveAnimationId(name);
    if (id === undefined) return;

    // Destroy previous session if exists
    if (is_editing()) {
        stop_editing();
    }

    currentExerciseName = name;
    currentExerciseId = id;

    // Look up animation data by ID
    const json = animationData[id];
    if (!json) {
        console.error(`No animation data for ${name} (ID: ${id})`);
        return;
    }

    set_exercise(id);
    load_animation(id, json);

    // Create new editor session with handle
    start_editing(id);


    currentKeyframe = 0;
    selectedJoint = null;
    clearHistory();

    updateUI();
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

    // Get joint positions using handle-based API
    try {
        const positions = getLogicalJointPositions();
        if (!positions || positions.length === 0) return;

        // Draw joints
        for (let i = 0; i < positions.length / 2; i++) {
            const x = positions[i * 2];
            const y = positions[i * 2 + 1];

            // Skip if behind camera
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
    } catch (e) {
        console.error('Overlay draw error:', e);
    }
}

function bindEvents(canvas: HTMLCanvasElement) {
    // Mouse/Touch
    canvas.addEventListener('pointermove', onPointerMove);
    canvas.addEventListener('pointerdown', onPointerDown);
    canvas.addEventListener('pointerup', onPointerUp);

    // Timeline Buttons
    document.getElementById('prev-kf')?.addEventListener('click', () => navigateKeyframe(-1));
    document.getElementById('next-kf')?.addEventListener('click', () => navigateKeyframe(1));
    document.getElementById('add-kf')?.addEventListener('click', addKeyframeHandler);
    document.getElementById('delete-kf')?.addEventListener('click', deleteKeyframeHandler);
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
    if (selectedJoint === null || !is_editing()) return;
    const target = e.target as HTMLInputElement;
    const id = target.id;
    const val = parseFloat(target.value);

    const info = get_bone_info(selectedJoint);
    if (info) {
        if (isNaN(val)) {
            // Restore the input to its current valid value
            set_bone_position(selectedJoint, info.x, info.y, info.z);
            set_bone_rotation(selectedJoint, info.rx, info.ry, info.rz);
        } else {
            if (id.startsWith('j-r')) {
                // Rotation
                let rx = id === 'j-rx' ? val : info.rx;
                let ry = id === 'j-ry' ? val : info.ry;
                let rz = id === 'j-rz' ? val : info.rz;
                set_bone_rotation(selectedJoint, rx, ry, rz);
            } else {
                // Position
                let x = id === 'j-x' ? val : info.x;
                let y = id === 'j-y' ? val : info.y;
                let z = id === 'j-z' ? val : info.z;
                set_bone_position(selectedJoint, x, y, z);
            }
        }
        info.free();
    }
}

function updateJointUI(jointId: number) {
    if (!is_editing()) return;
    const info = get_bone_info(jointId);

    const ph = document.getElementById('joint-placeholder');
    const ctrl = document.getElementById('joint-controls');
    const idSpan = document.getElementById('j-id');

    if (ph) ph.style.display = 'none';
    if (ctrl) ctrl.style.display = 'block';
    if (idSpan) idSpan.textContent = `${jointId} (${JOINT_NAMES[jointId] || 'Unknown'})`;

    const setVal = (id: string, v: number) => {
        const el = document.getElementById(id) as HTMLInputElement;
        if (el && document.activeElement !== el) {
            el.value = v.toFixed(2);
        }
    };

    if (info) {
        setVal('j-x', info.x);
        setVal('j-y', info.y);
        setVal('j-z', info.z);
        setVal('j-rx', info.rx);
        setVal('j-ry', info.ry);
        setVal('j-rz', info.rz);
        info.free();
    }
}

function onPointerMove(e: PointerEvent) {
    if (!currentExerciseName || !is_editing()) return;

    const rect = (e.target as HTMLElement).getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (!isDragging) {
        hoveredJoint = findNearestJoint(x, y);
        (e.target as HTMLElement).style.cursor = hoveredJoint !== null ? 'pointer' : 'default';
        updateUI();
    }

    if (isDragging && selectedJoint !== null) {
        const dx = e.clientX - dragStartX;
        const dy = e.clientY - dragStartY;

        // Get camera matrices for projection
        const view = getViewMatrix();
        const proj = getProjectionMatrix();
        const canvas = e.target as HTMLCanvasElement;
        const dpr = window.devicePixelRatio || 1;

        drag_joint(
            selectedJoint,
            dx * dpr,
            dy * dpr,
            view,
            proj,
            canvas.width,
            canvas.height
        );

        dragStartX = e.clientX;
        dragStartY = e.clientY;
    }
}

function onPointerDown(e: PointerEvent) {
    if (!currentExerciseName || !is_editing()) return;

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
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;

    if (e.key === '[') navigateKeyframe(-1);
    if (e.key === ']') navigateKeyframe(1);

    if (e.ctrlKey && e.key === 'z') undo();
    if (e.ctrlKey && (e.key === 'y' || (e.shiftKey && e.key === 'z'))) redo();

    if (e.key === 'Delete') deleteKeyframeHandler();
}

// --- Logic ---

function getLogicalJointPositions(): Float32Array | number[] | null {
    if (!is_editing()) return null;

    // Get camera matrices
    const view = getViewMatrix();
    const proj = getProjectionMatrix();
    const canvas = document.getElementById('gpu-canvas') as HTMLCanvasElement;
    if (!canvas) return null;

    const raw = get_joint_positions(view, proj, canvas.width, canvas.height);
    if (!raw || raw.length === 0) return null;

    // Scale down by DPR for logical CSS pixels
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
        let minDist = 20; // radius

        for (let i = 0; i < positions.length / 2; i++) {
            const jx = positions[i * 2];
            const jy = positions[i * 2 + 1];
            const d = Math.sqrt((jx - sx) ** 2 + (jy - sy) ** 2);
            if (d < minDist) {
                minDist = d;
                nearest = i;
            }
        }
        return nearest;
    } catch (e) {
        console.error('Find nearest joint error:', e);
        return null;
    }
}

function navigateKeyframe(delta: number) {
    if (!is_editing()) return;
    const count = get_keyframe_count();
    if (count === 0) return;

    currentKeyframe = Math.max(0, Math.min(count - 1, currentKeyframe + delta));
    set_keyframe_index(currentKeyframe);
    updateUI();
}

function addKeyframeHandler() {
    if (!is_editing()) return;
    saveUndoState();
    add_keyframe(currentKeyframe);
    currentKeyframe++;
    set_keyframe_index(currentKeyframe);
    updateUI();
}

function deleteKeyframeHandler() {
    if (!is_editing()) return;
    saveUndoState();
    delete_keyframe(currentKeyframe);
    const count = get_keyframe_count();
    if (currentKeyframe >= count) currentKeyframe = Math.max(0, count - 1);

    set_keyframe_index(currentKeyframe);
    updateUI();
}

function copyJson() {
    if (!is_editing()) return;
    try {
        const json = export_clip_json();
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

// --- UI Updates ---

function updateUI() {
    if (!is_editing()) {
        const timeDisplay = document.getElementById('time-display');
        if (timeDisplay) timeDisplay.textContent = 'No Animation';
        return;
    }

    const count = get_keyframe_count();
    const time = get_keyframe_time();

    const timeDisplay = document.getElementById('time-display');
    if (timeDisplay) timeDisplay.textContent = `${time.toFixed(2)} s (Frame ${currentKeyframe + 1}/${count})`;

    const statusText = document.getElementById('status-text');
    if (statusText) {
        statusText.textContent = selectedJoint !== null
            ? `Selected: ${JOINT_NAMES[selectedJoint]} `
            : (count > 0 ? 'Ready' : 'No Animation');
    }
}

// Start
init();
