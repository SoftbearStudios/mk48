in vec2 position;
in vec2 velocity;
in float color;
in float radius;
in float smoothness;
in float created;

uniform mat3 uView;
uniform vec4 uWind_uTime_uScale;

#ifdef SHADOWS
    uniform mat4 uShadowMatrix;
    uniform highp sampler2DShadow uShadow;
    uniform float altitude;
#endif

out vec4 vColor;
out float vSharpness;

void main() {
    float time = uWind_uTime_uScale.z - created;

    // position + velocity * (1 - 4^-t) / ln(4)
    #define LN_0_25 0.721347520444
    vec2 integratedPosition = position + velocity * (pow(0.25, time) * -LN_0_25 + LN_0_25) + uWind_uTime_uScale.xy * time * time;

    gl_Position = vec4(uView * vec3(integratedPosition, 1.0), 1.0);
    float life = smoothstep(0.0, 1.4, time);

    vec3 solidColor = vec3(color);
    if (color < 0.0) {
        // Fire to smoke.
        // color * 4^-t
        float integratedColor = color * pow(0.17, time);
        solidColor = mix(vec3(0.955, 0.523, 0.0), vec3(0.0), integratedColor + 1.0);
    }
    solidColor *= solidColor; // Pseudo gamma correction.

    float size = uWind_uTime_uScale.w * (1.0 + life * smoothness * 2.0) * radius;

    // Instead of making particles less than 1px, reduce their alpha.
    gl_PointSize = max(size, 2.0);
    float alpha = min(size * size * 0.25, 1.0) * ((1.0 - life) * (1.15 - smoothness));
    alpha *= 0.5 + float(color < 0.5); // TODO find a better way to define this effect.

    #ifdef SHADOWS
        // Sample shadow map once per particle.
        float light = textureProj(uShadow, uShadowMatrix * vec4(integratedPosition, altitude, 1.0)) * 0.85 + 0.15;
    #else
        float light = 1.0;
    #endif

    vColor = vec4(solidColor * light, alpha);
    vSharpness = (1.0 - smoothness) * 0.35 + 0.15;
}