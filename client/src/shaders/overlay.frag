precision mediump float;

varying vec2 vPosition;
uniform vec4 uMiddle_uDerivative;
uniform vec3 uAbove_uArea_uBorder;
uniform vec2 uRestrict_uVisual;

float preciseLength(vec2 vec) {
    #define LENGTH_SCALE 64.0
    return length(vec * (1.0 / LENGTH_SCALE)) * LENGTH_SCALE;
}

void main() {
    float area = (vPosition.y - uAbove_uArea_uBorder.y) * uAbove_uArea_uBorder.x;
    float border = preciseLength(vPosition) - uAbove_uArea_uBorder.z;
    gl_FragColor = mix(gl_FragColor, vec4(0.4, 0.15, 0.15, 1.0), clamp(max(border, area) * 0.1, 0.0, 0.5));
    gl_FragColor = mix(gl_FragColor, vec4(0.0, 0.14, 0.32, 1.0), clamp((preciseLength(vPosition - uMiddle_uDerivative.xy) - uRestrict_uVisual.y) * 0.1, 0.0, uRestrict_uVisual.x));
}