import type { CustomRenderMethodInput } from 'maplibre-gl';

export type TransitionProjection = () => CustomRenderMethodInput;
declare const transitionState: unique symbol;
export interface TransitionTarget { readonly [transitionState]: never; }
export declare function transitionTarget(
    gl: WebGL2RenderingContext,
    projection: TransitionProjection,
    paint: (properties: unknown) => readonly [unknown, unknown, unknown, unknown],
    water: (water: unknown) => readonly [unknown, unknown],
    cold: (destination: unknown) => void,
): TransitionTarget;
export declare function drawTransition(target: TransitionTarget, frame: unknown, shapes: readonly unknown[], fraction: number): void;
export declare function settleTransition(target: TransitionTarget, destination: unknown): void;
export declare function disposeTransition(target: TransitionTarget): void;
