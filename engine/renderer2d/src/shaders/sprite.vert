attribute vec2 position;
attribute vec2 uv;
attribute float alpha;
uniform mat3 uView;
varying vec2 vUv;
varying float vAlpha;

void main() {
    gl_Position = vec4(uView * vec3(position, 1.0), 1.0);
    vUv = uv;
    vAlpha = alpha;
}