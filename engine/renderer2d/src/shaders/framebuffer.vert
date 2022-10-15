attribute vec4 position;
attribute vec2 uv;
varying vec2 vUv;

void main() {
    gl_Position = position;
    vUv = vec2(uv * 0.5 + 0.5);
}
