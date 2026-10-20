import type { CustomRenderMethodInput } from 'maplibre-gl';

export type TransitionProjection = () => CustomRenderMethodInput;
export interface TransitionImageMarker {
    readonly image: Pick<ImageData, 'width' | 'height'> & { readonly data: Uint8Array | Uint8ClampedArray };
    readonly pixelRatio: number;
}
export interface TransitionPointMarker {
    readonly color: readonly number[];
    readonly size: number;
}
export type TransitionMarker = TransitionImageMarker | TransitionPointMarker;
declare const transitionState: unique symbol;
export interface TransitionTarget { readonly [transitionState]: never; }
export declare function transitionTarget<N>(
    gl: WebGL2RenderingContext,
    projection: TransitionProjection,
    paint: (properties: unknown) => readonly [unknown, unknown, TransitionMarker | null | undefined, unknown],
    water: (water: unknown) => readonly [unknown, unknown],
    cold: (destination: unknown) => void,
    identify: (reference: N) => string,
): TransitionTarget;
export declare function drawTransition(target: TransitionTarget, frame: unknown, shapes: readonly unknown[], fraction: number): void;
export declare function prepareTransition(target: TransitionTarget, collection: unknown): void;
export declare function resetTransition(target: TransitionTarget): void;
export declare function settleTransition(target: TransitionTarget, destination: unknown): void;
export declare function disposeTransition(target: TransitionTarget): void;
