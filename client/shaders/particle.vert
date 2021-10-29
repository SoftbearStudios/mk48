attribute vec2 position;
attribute float color;
attribute float alpha;
attribute float created;
uniform mat3 uView;
uniform float uTime;
uniform float uWindowSize;
varying vec3 vColor;
varying float life;

void main() {
    gl_Position = vec4(uView * vec3(position.x, position.y, 1.0), 1.0);
    life = smoothstep(0.0, 1.5, uTime - created);
    if (color < 0.0) {
        // Fire to smoke.
        vColor = mix(vec3(0.98, 0.75, 0.0), vec3(0.1), color + 1.0);
    } else {
        // Passthrough color.
        vColor = vec3(color);
    }
    gl_PointSize = uWindowSize * length((uView * vec3(1.0, 0.0, 0.0))) * (1.0 + life * 2.0);
}