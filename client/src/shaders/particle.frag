#version 300 es
precision mediump float;

in vec4 vColor;
in float vSharpness;

out vec4 fragColor;

void main() {
    float r = length(gl_PointCoord - vec2(0.5));
    fragColor = vec4(smoothstep(0.5, vSharpness, r) * vColor.a);
    fragColor.rgb *= vColor.rgb;
}