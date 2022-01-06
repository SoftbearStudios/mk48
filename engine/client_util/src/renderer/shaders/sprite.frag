precision mediump float;
varying vec2 vUv;
varying float vAlpha;
uniform sampler2D uSampler;

void main() {
    gl_FragColor = texture2D(uSampler, vUv) * vAlpha;
}