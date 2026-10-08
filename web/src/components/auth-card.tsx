import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card"

type Props = {
  title: string;
  description?: string;
  children: React.ReactNode;
};

export const AuthCard = ({ title, description, children }: Props) => {
  return (
    <Card className="w-full max-w-md">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>

      <CardContent>{children}</CardContent>
    </Card>
  );
}
