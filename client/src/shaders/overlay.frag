precision mediump float;

varying vec2 vPosition;
uniform vec4 uMiddle_uDerivative;
uniform vec3 uVisual_uRestrict_uBorder;

void main() {
    gl_FragColor = mix(gl_FragColor, vec4(0.4, 0.15, 0.15, 1.0), clamp((length(vPosition) - uVisual_uRestrict_uBorder.z) * 0.1, 0.0, 0.5));
    gl_FragColor = mix(gl_FragColor, vec4(0.0, 0.14, 0.32, 1.0), clamp((length(vPosition - uMiddle_uDerivative.xy) - uVisual_uRestrict_uBorder.x) * 0.1, 0.0, uVisual_uRestrict_uBorder.y));
}