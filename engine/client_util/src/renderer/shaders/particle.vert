attribute vec2 position;
attribute vec2 velocity;
attribute float color;
attribute float radius;
attribute float smoothness;
attribute float created;
uniform mat3 uView;
uniform vec4 uWind_uTime_uScale;
varying vec4 vColor;
varying float vSmoothness;

void main() {
    float time = uWind_uTime_uScale.z - created;
    float pow0_25Time = pow(0.25, time);

    // position + velocity * (1 - 4^-t) / ln(4)
    #define LN_0_25 0.721347520444
    vec2 integratedPosition = position + velocity * (pow0_25Time * -LN_0_25 + LN_0_25) + uWind_uTime_uScale.xy * time * time;

    gl_Position = vec4(uView * vec3(integratedPosition, 1.0), 1.0);
    float life = smoothstep(0.0, 1.4, time);

    vec3 solidColor = vec3(color);
    if (color < 0.0) {
        // Fire to smoke.
        // color * 4^-t
        float integratedColor = color * pow0_25Time;
        solidColor = mix(vec3(0.98, 0.75, 0.0), vec3(0.1), integratedColor + 1.0);
    }

    float size = uWind_uTime_uScale.w * (1.0 + life * smoothness * 2.0) * radius;

    // Instead of making particles less than 1px, reduce their alpha.
    gl_PointSize = max(size, 1.0);
    float alpha = min(size * size, 1.0) * ((1.0 - life) * (1.15 - smoothness));

    vColor = vec4(solidColor, alpha);
    vSmoothness = smoothness;
}