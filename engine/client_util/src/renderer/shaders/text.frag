precision mediump float;
varying vec2 vUv;
uniform sampler2D uSampler;
uniform vec4 uColor;

void main() {
    gl_FragColor = texture2D(uSampler, vUv) * uColor;

    // Premultiply alpha.
    gl_FragColor.rgb *= uColor.a;
}