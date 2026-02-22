export function calculateBottomObstructionInset(
    layoutViewportHeight: number,
    visualViewportHeight: number,
    visualViewportOffsetTop: number
): number {
    const bottomObstruction = layoutViewportHeight - (visualViewportHeight + visualViewportOffsetTop);
    return Math.max(0, Math.round(bottomObstruction));
}

type BottomInsetOptions = {
    layoutViewportHeight: number;
    visualViewportHeight?: number;
    visualViewportOffsetTop?: number;
    isAndroid: boolean;
    isFullscreen: boolean;
    androidMinimumInset?: number;
};

export function calculateEffectiveBottomInset(options: BottomInsetOptions): number {
    const visualHeight = options.visualViewportHeight ?? options.layoutViewportHeight;
    const visualOffsetTop = options.visualViewportOffsetTop ?? 0;
    const obstructionInset = calculateBottomObstructionInset(
        options.layoutViewportHeight,
        visualHeight,
        visualOffsetTop
    );

    const androidMinimumInset = options.androidMinimumInset ?? 56;
    const minimumInset = options.isAndroid && !options.isFullscreen ? androidMinimumInset : 0;

    return Math.max(obstructionInset, minimumInset);
}
