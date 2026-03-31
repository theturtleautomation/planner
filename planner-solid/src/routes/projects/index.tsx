import { Navigate } from "@solidjs/router";

import { withFrontendMockSearch } from "~/lib/mock/runtime";

export default function ProjectsPage() {
  return <Navigate href={withFrontendMockSearch("/")} />;
}
