attribute vec4 position;
uniform vec2 uVP;
varying vec2 vUv;

void main() {
    gl_Position = position;
    vUv = vec2(position * 0.5 + 0.5) * uVP;
}
