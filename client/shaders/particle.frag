precision mediump float;
varying vec3 vColor;
varying float life;

void main() {
    float r = length(gl_PointCoord - vec2(0.5));
    float a = 0.2 - life * 0.2;
    gl_FragColor = vec4((1.0 - smoothstep(0.0, 0.5, r)) * a);
    gl_FragColor.rgb *= vColor;
}