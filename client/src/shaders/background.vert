attribute vec4 position;
attribute vec2 uv;
uniform mat3 uCamera;
uniform mat3 uTexture;
varying vec2 vPosition; // world position.
varying vec2 vUv; // terrain texture uv.
varying vec2 vUv2; // sand/grass texture uv.

void main() {
    gl_Position = position;
    vPosition = (uCamera * vec3(uv, 1.0)).xy;
    vUv = (uTexture * vec3(vPosition, 1.0)).xy;
    vUv2 = vPosition * 0.005;
}
