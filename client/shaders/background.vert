attribute vec4 position;
uniform mat3 uView;
varying vec2 vPosition; // world position.


void main() {
    gl_Position = position;
    vPosition = (uView * vec3(position.xy, 1.0)).xy;
}
