attribute vec2 position;
attribute vec4 color;
uniform mat3 uView;
varying vec4 vColor;

void main() {
    gl_Position = vec4(uView * vec3(position.x, position.y, 1.0), 1.0);
    vColor = color;

    // Premultiply alpha.
    vColor.rgb *= color.a;
}