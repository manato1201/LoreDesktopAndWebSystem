"use client";

import { Canvas } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";

const JOINTS: [number, number, number][] = [
  [0, 1.25, 0],
  [-0.5, 0.68, 0],
  [0.5, 0.68, 0],
  [-0.18, -0.3, 0],
  [0.18, -0.3, 0],
];

function Rig({ color }: { color: string }) {
  return (
    <group>
      <mesh position={[0, 1.6, 0]}>
        <sphereGeometry args={[0.28, 20, 20]} />
        <meshStandardMaterial color={color} />
      </mesh>
      <mesh position={[0, 0.9, 0]}>
        <capsuleGeometry args={[0.28, 0.6, 4, 12]} />
        <meshStandardMaterial color={color} />
      </mesh>
      <mesh position={[-0.5, 0.95, 0]} rotation={[0, 0, Math.PI / 2.3]}>
        <capsuleGeometry args={[0.1, 0.55, 4, 8]} />
        <meshStandardMaterial color={color} />
      </mesh>
      <mesh position={[0.5, 0.95, 0]} rotation={[0, 0, -Math.PI / 2.3]}>
        <capsuleGeometry args={[0.1, 0.55, 4, 8]} />
        <meshStandardMaterial color={color} />
      </mesh>
      <mesh position={[-0.18, 0.05, 0]}>
        <capsuleGeometry args={[0.12, 0.7, 4, 8]} />
        <meshStandardMaterial color={color} />
      </mesh>
      <mesh position={[0.18, 0.05, 0]}>
        <capsuleGeometry args={[0.12, 0.7, 4, 8]} />
        <meshStandardMaterial color={color} />
      </mesh>
      {JOINTS.map((position, index) => (
        <mesh key={index} position={position}>
          <sphereGeometry args={[0.07, 12, 12]} />
          <meshStandardMaterial
            color="#1ed760"
            emissive="#1ed760"
            emissiveIntensity={0.4}
          />
        </mesh>
      ))}
    </group>
  );
}

/**
 * There is no real binary asset to stream in this mock backend, so this
 * renders a stand-in humanoid rig rather than loading actual FBX/GLTF
 * chunk data. `variant` only shifts the body color so Before/After diff
 * views read as visually distinct.
 */
export function ModelViewer({
  variant,
  className,
}: {
  variant?: "before" | "after";
  className?: string;
}) {
  const bodyColor = variant === "after" ? "#9a9a9a" : "#5f5f5f";

  return (
    <div
      className={`overflow-hidden rounded-comfortable bg-surface-elevated ${className ?? ""}`}
    >
      <Canvas camera={{ position: [1.6, 1.4, 2.2], fov: 45 }} dpr={[1, 1.5]}>
        <color attach="background" args={["#181818"]} />
        <ambientLight intensity={0.6} />
        <directionalLight position={[3, 4, 2]} intensity={1.1} />
        <Rig color={bodyColor} />
        <gridHelper
          args={[4, 8, "#2a2a2a", "#252525"]}
          position={[0, -0.35, 0]}
        />
        <OrbitControls
          enablePan={false}
          minDistance={1.5}
          maxDistance={5}
          target={[0, 0.8, 0]}
        />
      </Canvas>
    </div>
  );
}
