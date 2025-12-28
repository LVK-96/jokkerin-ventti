import { describe, it, expect, vi, beforeEach } from 'vitest';
import { 
  initHistory, 
  saveUndoState, 
  undo, 
  redo, 
  clearHistory, 
  HistoryCallbacks 
} from './history';

// 1. Mock the WASM functions
vi.mock('../../wasm/pkg/jokkerin_ventti_wasm', () => ({
  export_animation_json: vi.fn(),
  load_animation: vi.fn(),
  enter_editor_mode: vi.fn(),
  set_editor_keyframe: vi.fn(),
}));

describe('History System', () => {
  let callbacks: HistoryCallbacks;
  
  // Mocks for callbacks
  let mockGetPoseJson = vi.fn();
  let mockGetKeyframeIndex = vi.fn();
  let mockGetSelectedJoint = vi.fn();
  let mockLoadPose = vi.fn();
  let mockSetKeyframeIndex = vi.fn();
  let mockSetSelectedJoint = vi.fn();
  let mockOnHistoryRestore = vi.fn();

  beforeEach(() => {
    clearHistory();
    vi.clearAllMocks();

    // Default mock returns
    mockGetPoseJson.mockReturnValue('{}');
    mockGetKeyframeIndex.mockReturnValue(0);
    mockGetSelectedJoint.mockReturnValue(null);

    callbacks = {
      getPoseJson: mockGetPoseJson,
      getKeyframeIndex: mockGetKeyframeIndex,
      getSelectedJoint: mockGetSelectedJoint,
      loadPose: mockLoadPose,
      setKeyframeIndex: mockSetKeyframeIndex,
      setSelectedJoint: mockSetSelectedJoint,
      onHistoryRestore: mockOnHistoryRestore,
    };

    initHistory(callbacks);
  });

  it('starts empty (undo/redo do nothing)', () => {
    // 2. Test that history starts empty
    undo();
    expect(mockLoadPose).not.toHaveBeenCalled();
    
    redo();
    expect(mockLoadPose).not.toHaveBeenCalled();
  });

  it('enables undo after saving state', () => {
    // 3. Test saveUndoState enables undo
    mockGetPoseJson.mockReturnValue('{"state":1}');
    saveUndoState();

    // Simulate change
    mockGetPoseJson.mockReturnValue('{"state":2}');

    undo();
    
    expect(mockLoadPose).toHaveBeenCalledWith('{"state":1}');
    expect(mockOnHistoryRestore).toHaveBeenCalled();
  });

  it('clears redo stack when saving state', () => {
    // 4. Test undo followed by saveUndoState clears redo stack
    
    // Save State 1
    mockGetPoseJson.mockReturnValue('state1');
    saveUndoState();

    // Move to State 2
    mockGetPoseJson.mockReturnValue('state2');

    // Undo -> Moves State 2 to Redo, Restores State 1
    undo();
    expect(mockLoadPose).toHaveBeenCalledWith('state1');

    // Verify Redo works (State 2 should be there)
    // When we redo, we save current (State 1) to Undo, and restore State 2
    mockGetPoseJson.mockReturnValue('state1'); 
    redo();
    expect(mockLoadPose).toHaveBeenCalledWith('state2');

    // Undo again to get back to State 1
    mockGetPoseJson.mockReturnValue('state2');
    undo();
    expect(mockLoadPose).toHaveBeenCalledWith('state1');

    // Now at State 1. Redo stack has State 2.
    // Save new State (State 3)
    mockGetPoseJson.mockReturnValue('state3');
    saveUndoState();

    // Redo should now be empty (State 2 lost)
    mockLoadPose.mockClear();
    redo();
    expect(mockLoadPose).not.toHaveBeenCalled();
  });

  it('restores state correctly on undo', () => {
    // 5. Test state restoration on undo
    mockGetPoseJson.mockReturnValue('correct-json');
    mockGetKeyframeIndex.mockReturnValue(42);
    mockGetSelectedJoint.mockReturnValue(7);
    
    saveUndoState();

    // Change everything
    mockGetPoseJson.mockReturnValue('wrong-json');
    mockGetKeyframeIndex.mockReturnValue(0);
    mockGetSelectedJoint.mockReturnValue(null);

    undo();

    expect(mockLoadPose).toHaveBeenCalledWith('correct-json');
    expect(mockSetKeyframeIndex).toHaveBeenCalledWith(42);
    expect(mockSetSelectedJoint).toHaveBeenCalledWith(7);
  });
});
