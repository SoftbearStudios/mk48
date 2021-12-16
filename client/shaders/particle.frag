precision mediump float;
varying vec4 vColor;

void main() {
    float r = length(gl_PointCoord - vec2(0.5));
    gl_FragColor = vec4(smoothstep(0.5, 0.25, r) * vColor.a);
    gl_FragColor.rgb *= vColor.rgb;
}