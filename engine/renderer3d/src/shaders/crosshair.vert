attribute vec2 position;
uniform vec2 uScale;

void main() {
    gl_Position = vec4(position * uScale, -1.0, 1.0);
}