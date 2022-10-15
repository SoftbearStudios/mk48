precision mediump float;

varying vec2 vUv;
uniform sampler2D uSampler;

// Based on https://github.com/gfx-rs/wgpu/blob/master/wgpu-hal/src/gles/shaders/srgb_present.frag
void main() {
    vec4 linear = texture2D(uSampler, vUv);
    vec3 color_linear = linear.rgb;
    vec3 selector = ceil(color_linear - 0.0031308);
    vec3 under = 12.92 * color_linear;
    vec3 over = 1.055 * pow(color_linear, vec3(0.41666)) - 0.055;
    vec3 result = mix(under, over, selector);
    gl_FragColor = vec4(result, linear.a);
}
