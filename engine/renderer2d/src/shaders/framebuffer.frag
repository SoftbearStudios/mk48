precision mediump float;

varying highp vec2 vUv;
uniform sampler2D uSampler;

void main() {
    gl_FragColor = texture2D(uSampler, vUv);
}
