export function calculateBottomObstructionInset(
    layoutViewportHeight: number,
    visualViewportHeight: number,
    visualViewportOffsetTop: number
): number {
    const bottomObstruction = layoutViewportHeight - (visualViewportHeight + visualViewportOffsetTop);
    return Math.max(0, Math.round(bottomObstruction));
}

