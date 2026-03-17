import { useRef, useMemo, useEffect, useState } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import * as THREE from "three";

// Seeded pseudo-random number generator for deterministic rendering
function createSeededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = Math.sin(s * 12.9898 + 78.233) * 43758.5453;
    return s - Math.floor(s);
  };
}

function Particles({ count = 1500 }: { count?: number }) {
  const mesh = useRef<THREE.Points>(null);
  const mouse = useRef({ x: 0, y: 0 });

  const [positions, colors] = useMemo(() => {
    const random = createSeededRandom(12345);
    const positions = new Float32Array(count * 3);
    const colors = new Float32Array(count * 3);

    for (let i = 0; i < count; i++) {
      // Create a spherical distribution
      const r = 12 * Math.cbrt(random());
      const theta = random() * 2 * Math.PI;
      const phi = Math.acos(2 * random() - 1);

      positions[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      positions[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      positions[i * 3 + 2] = r * Math.cos(phi);

      // Cool color palette (Cyan, Blue, Slate)
      const colorChoice = random();
      if (colorChoice < 0.4) {
        // Cyan-ish
        colors[i * 3] = 0.0;
        colors[i * 3 + 1] = 0.8;
        colors[i * 3 + 2] = 1.0;
      } else if (colorChoice < 0.8) {
        // Blue
        colors[i * 3] = 0.1;
        colors[i * 3 + 1] = 0.4;
        colors[i * 3 + 2] = 1.0;
      } else {
        // Slate / Darker blue
        colors[i * 3] = 0.3;
        colors[i * 3 + 1] = 0.4;
        colors[i * 3 + 2] = 0.6;
      }
    }

    return [positions, colors];
  }, [count]);

  useFrame((state) => {
    if (!mesh.current) return;

    const time = state.clock.getElapsedTime();
    mesh.current.rotation.x = time * 0.02;
    mesh.current.rotation.y = time * 0.03;

    // Mouse interaction parallax
    const { x, y } = state.pointer;
    mouse.current.x += (x - mouse.current.x) * 0.05;
    mouse.current.y += (y - mouse.current.y) * 0.05;

    mesh.current.position.x = mouse.current.x * 2;
    mesh.current.position.y = mouse.current.y * 2;
  });

  return (
    <points ref={mesh}>
      <bufferGeometry>
        <bufferAttribute attach="attributes-position" args={[positions, 3]} />
        <bufferAttribute attach="attributes-color" args={[colors, 3]} />
      </bufferGeometry>
      <pointsMaterial size={0.06} vertexColors transparent opacity={0.6} sizeAttenuation />
    </points>
  );
}

function WireframeSphere() {
  const meshRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (!meshRef.current) return;
    const time = state.clock.getElapsedTime();
    meshRef.current.rotation.y = time * 0.05;
    meshRef.current.rotation.x = time * 0.02;
  });

  return (
    <mesh ref={meshRef}>
      <icosahedronGeometry args={[10, 2]} />
      <meshBasicMaterial color="#0ea5e9" wireframe transparent opacity={0.1} />
    </mesh>
  );
}

export function ParticleField() {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return (
      <div className="pointer-events-none fixed inset-0 z-0 bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950" />
    );
  }

  return (
    <div className="pointer-events-none fixed inset-0 z-0">
      <Canvas
        camera={{ position: [0, 0, 15], fov: 60 }}
        dpr={[1, 1.5]}
        gl={{ antialias: false, alpha: true }}
      >
        <ambientLight intensity={0.5} />
        <Particles count={1500} />
        <WireframeSphere />
      </Canvas>
    </div>
  );
}
