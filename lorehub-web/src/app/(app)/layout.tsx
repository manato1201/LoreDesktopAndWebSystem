import { redirect } from "next/navigation";
import { Sidebar } from "@/components/Sidebar";
import { ToastProvider } from "@/components/Toast";
import { getCurrentUser } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export default async function AppLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const cookie = await getSessionCookieHeader();
  const user = cookie ? await getCurrentUser(cookie) : null;

  if (!user) {
    redirect("/login");
  }

  return (
    <ToastProvider>
      <div className="flex min-h-full">
        <Sidebar user={user} />
        <main className="min-w-0 flex-1 px-8 py-6">{children}</main>
      </div>
    </ToastProvider>
  );
}
