precision mediump float;
varying vec4 vColor;
varying float vSharpness;

void main() {
    float r = length(gl_PointCoord - vec2(0.5));
    gl_FragColor = vec4(smoothstep(0.5, vSharpness, r) * vColor.a);
    gl_FragColor.rgb *= vColor.rgb;
}