attribute vec2 position;
attribute vec2 velocity;
attribute float color;
attribute float radius;
attribute float created;
uniform mat3 uView;
uniform vec2 uWind;
uniform float uTime;
uniform float uWindowSize;
varying vec4 vColor;

void main() {
    float time = uTime - created;
    float pow0_25Time = pow(0.25, time);

    // position + velocity * (1 - 4^-t) / ln(4)
    #define LN_0_25 0.721347520444
    vec2 integratedPosition = position + velocity * (pow0_25Time * -LN_0_25 + LN_0_25) + uWind * time;

    gl_Position = vec4(uView * vec3(integratedPosition, 1.0), 1.0);
    float life = smoothstep(0.0, 1.4, time);

    vec3 solidColor = vec3(color);
    if (color < 0.0) {
        // Fire to smoke.
        // color * 4^-t
        float integratedColor = color * pow0_25Time;
        solidColor = mix(vec3(0.98, 0.75, 0.0), vec3(0.1), integratedColor + 1.0);
    }

    float size = uWindowSize * length((uView * vec3(1.0, 0.0, 0.0))) * (1.0 + life * 2.0) * radius;

    // Instead of making particles less than 1px, reduce their alpha.
    gl_PointSize = max(size, 1.0);
    float alpha = min(size * size, 1.0) * ((1.0 - life) * 0.15);

    vColor = vec4(solidColor, alpha);
}