attribute vec2 position;
attribute float created;
uniform mat3 uView;
uniform float uTime;
varying float life;

void main() {
    gl_Position = vec4(uView * vec3(position.x, position.y, 1.0), 1.0);
    life = smoothstep(0.0, 1.5, uTime - created);
    gl_PointSize = 1080.0 * length((uView * vec3(1.0, 0.0, 0.0))) * (1.0 + life * 2.0);
}