#version 300 es

in vec4 position;
in vec2 uv;
in float alpha;
in vec2 tangent;

uniform mat3 uView;

out vec4 vPosition;
out vec2 vUv;
out vec2 vColor;
out vec3 vTangent;

void main() {
    vPosition = position;
    gl_Position = vec4(uView * vec3(position.xy, 1.0), 1.0);
    vUv = uv;
    vColor = alpha == 0.0 ? vec2(0.0, 0.48) : vec2(alpha); // TODO fix drop shadows blending towards 0
    vTangent = vec3(tangent, 0.0);
}
