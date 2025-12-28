export interface UndoState {
    keyframeIndex: number;
    poseJson: string;
    selectedJoint: number | null;
}

export interface HistoryCallbacks {
    getKeyframeIndex: () => number;
    getPoseJson: () => string;
    getSelectedJoint: () => number | null;
    loadPose: (json: string) => void;
    setKeyframeIndex: (index: number) => void;
    setSelectedJoint: (index: number | null) => void;
    onHistoryRestore: () => void;
}

const undoStack: UndoState[] = [];
const redoStack: UndoState[] = [];
const MAX_UNDO_STATES = 10;
let callbacks: HistoryCallbacks | null = null;

export function initHistory(cb: HistoryCallbacks) {
    callbacks = cb;
}

export function clearHistory() {
    undoStack.length = 0;
    redoStack.length = 0;
}

export function saveUndoState() {
    if (!callbacks) return;
    try {
        const json = callbacks.getPoseJson();
        undoStack.push({
            keyframeIndex: callbacks.getKeyframeIndex(),
            poseJson: json,
            selectedJoint: callbacks.getSelectedJoint()
        });
        if (undoStack.length > MAX_UNDO_STATES) undoStack.shift();
        redoStack.length = 0;
    } catch (e) { console.error('saveUndoState failed:', e); }
}

export function undo() {
    if (!callbacks || !undoStack.length) return;
    try {
        const currentHook = callbacks.getPoseJson();
        redoStack.push({
            keyframeIndex: callbacks.getKeyframeIndex(),
            poseJson: currentHook,
            selectedJoint: callbacks.getSelectedJoint()
        });

        const state = undoStack.pop()!;
        restoreState(state);
    } catch (e) { console.error('undo failed:', e); }
}

export function redo() {
    if (!callbacks || !redoStack.length) return;
    try {
        const currentHook = callbacks.getPoseJson();
        undoStack.push({
            keyframeIndex: callbacks.getKeyframeIndex(),
            poseJson: currentHook,
            selectedJoint: callbacks.getSelectedJoint()
        });

        const state = redoStack.pop()!;
        restoreState(state);
    } catch (e) { console.error('redo failed:', e); }
}

function restoreState(state: UndoState) {
    if (!callbacks) return;
    callbacks.loadPose(state.poseJson);
    callbacks.setKeyframeIndex(state.keyframeIndex);
    callbacks.setSelectedJoint(state.selectedJoint);
    callbacks.onHistoryRestore();
}
