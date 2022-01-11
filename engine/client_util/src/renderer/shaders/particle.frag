precision mediump float;
varying vec4 vColor;
varying float vSmoothness;

void main() {
    float r = length(gl_PointCoord - vec2(0.5)) * vSmoothness;
    gl_FragColor = vec4(smoothstep(0.5, 0.15, r) * vColor.a);
    gl_FragColor.rgb *= vColor.rgb;
}