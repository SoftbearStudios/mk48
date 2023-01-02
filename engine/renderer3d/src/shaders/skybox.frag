precision mediump float;

varying vec4 vUv;
uniform samplerCube uSampler;

void main() {
    gl_FragColor = textureCube(uSampler, normalize(vUv.xyz / vUv.w));
}
