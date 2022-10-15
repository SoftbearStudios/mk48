attribute vec2 position;
uniform mat3 uModelView;
varying vec2 vUv;

void main() {
    gl_Position = vec4(uModelView * vec3(position, 1.0), 1.0);
    vUv = vec2(position.x + 0.5, 0.5 - position.y);
}