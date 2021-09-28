precision mediump float;
varying vec2 vUv;
varying vec4 vColor;
uniform sampler2D uSampler;

void main() {
    gl_FragColor = texture2D(uSampler, vUv) * vColor;

    // Premultiply alpha.
    gl_FragColor.rgb *= vColor.a;
}